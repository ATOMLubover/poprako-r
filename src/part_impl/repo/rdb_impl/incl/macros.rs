// Generates BatchByIds loader structs for each declared table.
macro_rules! preload_by_ids {
    (
        $(
            $loader:ident {
                row: $row:ty,
                info: $info:ty,
                table: $table:ident,
                convert: $convert:ident,
            }
        )*
    ) => {
        $(
            pub struct $loader;

            impl BatchByIds for $loader {
                type Row = $row;
                type Info = $info;

                #[tracing::instrument(level = "info", skip_all)]
                async fn load(
                    conn: &mut RdbConn,
                    ids: Vec<&str>,
                ) -> BaseRest<Vec<$row>> {
                    $table::table
                        .filter($table::f_id.eq_any(ids))
                        .select(<$row>::as_select())
                        .load(conn)
                        .await
                        .map_err(diesel)
                }

                fn into_entry(row: $row) -> BaseRest<(String, $info)> {
                    let id = row.f_id.clone();

                    let info: $info = preload_by_ids!(@convert $convert $info, row);

                    Ok((id, info))
                }
            }
        )*
    };

    (@convert From $info:ty, $row:ident) => {
        <$info>::from($row)
    };

    (@convert TryFrom $info:ty, $row:ident) => {
        <$info>::try_from($row)?
    };
}

// Resolves a nested foreign-key value through an optional chain path.
macro_rules! preloadable_resolve_path {
    ($owner:expr; [] => $field:ident) => {
        Some($owner.$field.as_str())
    };

    ($owner:expr; [$head:ident $(, $tail:ident)*] => $field:ident) => {
        $owner.$head.as_ref().and_then(|owner| {
            preloadable_resolve_path!(owner; [$($tail),*] => $field)
        })
    };
}

// Injects a loaded related entity into an owner through a nested optional path.
macro_rules! preloadable_inject_path {
    ($owner:ident, $related:ident; [] => $field:ident) => {
        $owner.$field = $related;
    };

    ($owner:ident, $related:ident; [$head:ident $(, $tail:ident)*] => $field:ident) => {
        let Some($owner) = $owner.$head.as_mut() else {
            return;
        };

        preloadable_inject_path!($owner, $related; [$($tail),*] => $field);
    };
}

// Generates an Incl struct for one include variant with resolve/inject logic.
macro_rules! preloadable_variant {
    (
        $owner:ty;
        $marker:ident => $query:ident {
            resolve: path [$($resolve_path:ident),*] => $resolve_field:ident,
            inject: path [$($inject_path:ident),*] => $inject_field:ident,
        }
    ) => {
        preloadable_variant!(@generate $owner; $marker => $query {
            resolve: |owner| {
                preloadable_resolve_path!(owner; [$($resolve_path),*] => $resolve_field)
            },
            inject: |owner, related| {
                preloadable_inject_path!(owner, related; [$($inject_path),*] => $inject_field);
            },
        });
    };

    (
        $owner:ty;
        $marker:ident => $query:ident {
            resolve: with |$resolve_owner:ident| $resolve:expr,
            inject: path [$($inject_path:ident),*] => $inject_field:ident,
        }
    ) => {
        preloadable_variant!(@generate $owner; $marker => $query {
            resolve: |$resolve_owner| { $resolve },
            inject: |owner, related| {
                preloadable_inject_path!(owner, related; [$($inject_path),*] => $inject_field);
            },
        });
    };

    (
        $owner:ty;
        $marker:ident => $query:ident {
            resolve: path [$($resolve_path:ident),*] => $resolve_field:ident,
            inject: with |$inject_owner:ident, $inject_related:ident| $inject:expr,
        }
    ) => {
        preloadable_variant!(@generate $owner; $marker => $query {
            resolve: |owner| {
                preloadable_resolve_path!(owner; [$($resolve_path),*] => $resolve_field)
            },
            inject: |$inject_owner, $inject_related| { $inject; },
        });
    };

    (
        $owner:ty;
        $marker:ident => $query:ident {
            resolve: with |$resolve_owner:ident| $resolve:expr,
            inject: with |$inject_owner:ident, $inject_related:ident| $inject:expr,
        }
    ) => {
        preloadable_variant!(@generate $owner; $marker => $query {
            resolve: |$resolve_owner| { $resolve },
            inject: |$inject_owner, $inject_related| { $inject; },
        });
    };

    (@generate
        $owner:ty;
        $marker:ident => $query:ident {
            resolve: |$resolve_owner:ident| $resolve:block,
            inject: |$inject_owner:ident, $inject_related:ident| $inject:block,
        }
    ) => {
        struct $marker;

        impl Incl for $marker {
            type Owner = $owner;
            type Related = <$query as BatchByIds>::Info;
            type Query = $query;

            fn resolve_key(owner: &Self::Owner) -> Option<&str> {
                let $resolve_owner = owner;

                $resolve
            }

            fn inject(owner: &mut Self::Owner, related: Option<Self::Related>) {
                let $inject_owner = owner;
                let $inject_related = related;

                $inject
            }
        }
    };
}

// Generates include-variant structs and a top-level populate function.
macro_rules! preloadable {
    (
        owner: $owner:ty,
        opt: $opt:ident,
        populate: $populate:ident,
        variants: {
            $(
                $marker:ident => $query:ident { $($body:tt)* },
            )*
        },
    ) => {
        $(
            preloadable_variant!($owner; $marker => $query { $($body)* });
        )*

        #[tracing::instrument(level = "info", skip_all)]
        pub async fn $populate(
            conn: &mut RdbConn,
            infos: &mut [$owner],
            incl_opt: &[$opt],
        ) -> BaseRest<()> {
            for incl_opt in expand_incl_opts(incl_opt) {
                match incl_opt {
                    $(
                        $opt::$marker => populate::<$marker>(conn, infos).await?,
                    )*
                }
            }

            Ok(())
        }
    };
}
