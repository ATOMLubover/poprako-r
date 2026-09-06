// @generated automatically by Diesel CLI.

diesel::table! {
    t_announcement (f_id) {
        f_id -> Text,
        f_team_id -> Text,
        f_user_id -> Text,
        f_title -> Text,
        f_content -> Text,
        f_created_at -> Timestamptz,
    }
}

diesel::table! {
    t_assignment (f_id) {
        f_id -> Text,
        f_chapter_id -> Text,
        f_user_id -> Text,
        f_assigned_raw_provider_at -> Nullable<Timestamptz>,
        f_assigned_translator_at -> Nullable<Timestamptz>,
        f_assigned_proofreader_at -> Nullable<Timestamptz>,
        f_assigned_typesetter_at -> Nullable<Timestamptz>,
        f_assigned_redrawer_at -> Nullable<Timestamptz>,
        f_assigned_reviewer_at -> Nullable<Timestamptz>,
        f_assigned_publisher_at -> Nullable<Timestamptz>,
        f_assigned_admin_at -> Nullable<Timestamptz>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_assignment_invitation (f_id) {
        f_id -> Text,
        f_chapter_id -> Text,
        f_inviter_id -> Text,
        f_invitee_qid -> Text,
        f_code -> Text,
        f_pending -> Bool,
        f_role_mask -> Int8,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_chapter (f_id) {
        f_id -> Text,
        f_comic_id -> Text,
        f_is_pinned -> Bool,
        f_index -> Int4,
        f_subtitle -> Text,
        f_page_count -> Int4,
        f_total_unit_count -> Int4,
        f_translated_unit_count -> Int4,
        f_proofread_unit_count -> Int4,
        f_uploaded_at -> Nullable<Timestamptz>,
        f_translating_at -> Nullable<Timestamptz>,
        f_translated_at -> Nullable<Timestamptz>,
        f_proofreading_at -> Nullable<Timestamptz>,
        f_proofread_at -> Nullable<Timestamptz>,
        f_typesetting_at -> Nullable<Timestamptz>,
        f_typeset_at -> Nullable<Timestamptz>,
        f_reviewed_at -> Nullable<Timestamptz>,
        f_published_at -> Nullable<Timestamptz>,
        f_creator_id -> Text,
        f_deleted_at -> Nullable<Timestamptz>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_chapter_artwork (f_id) {
        f_id -> Text,
        f_version -> Int8,
        f_key -> Nullable<Text>,
        f_is_uploaded -> Nullable<Bool>,
        f_hash -> Nullable<Bytea>,
        f_ext -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_chapter_workflow_record (f_id) {
        f_id -> Text,
        f_chapter_id -> Text,
        f_actor_user_id -> Nullable<Text>,
        f_kind -> Text,
        f_payload -> Jsonb,
        f_created_at -> Timestamptz,
    }
}

diesel::table! {
    t_comic (f_id) {
        f_id -> Text,
        f_workset_id -> Text,
        f_index -> Int4,
        f_title -> Text,
        f_author -> Text,
        f_description -> Nullable<Text>,
        f_composed_title -> Text,
        f_chapter_count -> Int4,
        f_chapter_next_index -> Int4,
        f_creator_id -> Text,
        f_last_active_at -> Timestamptz,
        f_archived_at -> Nullable<Timestamptz>,
        f_deleted_at -> Nullable<Timestamptz>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_comic_archive (f_id) {
        f_id -> Text,
        f_team_id -> Text,
        f_source_comic_id -> Text,
        f_archived_payload -> Text,
        f_archiver_id -> Text,
        f_created_at -> Timestamptz,
    }
}

diesel::table! {
    t_comic_cover (f_id) {
        f_id -> Text,
        f_version -> Int8,
        f_key -> Nullable<Text>,
        f_is_uploaded -> Nullable<Bool>,
        f_hash -> Nullable<Bytea>,
        f_ext -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_comment (f_id) {
        f_id -> Text,
        f_team_id -> Text,
        f_user_id -> Text,
        f_content -> Text,
        f_created_at -> Timestamptz,
    }
}

diesel::table! {
    t_local_message (f_id) {
        f_id -> Text,
        f_topic -> Text,
        f_status -> Text,
        f_payload -> Jsonb,
        f_last_error -> Nullable<Text>,
        f_retried_count -> Int8,
        f_lease -> Int8,
        f_visible_at -> Timestamptz,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_member (f_id) {
        f_id -> Text,
        f_user_id -> Text,
        f_user_nickname -> Text,
        f_team_id -> Text,
        f_assigned_raw_provider_at -> Nullable<Timestamptz>,
        f_assigned_translator_at -> Nullable<Timestamptz>,
        f_assigned_proofreader_at -> Nullable<Timestamptz>,
        f_assigned_typesetter_at -> Nullable<Timestamptz>,
        f_assigned_redrawer_at -> Nullable<Timestamptz>,
        f_assigned_reviewer_at -> Nullable<Timestamptz>,
        f_assigned_publisher_at -> Nullable<Timestamptz>,
        f_assigned_admin_at -> Nullable<Timestamptz>,
        f_assigned_bot_at -> Nullable<Timestamptz>,
        f_user_last_active_at -> Timestamptz,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_member_invitation (f_id) {
        f_id -> Text,
        f_inviter_id -> Text,
        f_team_id -> Text,
        f_invitee_qid -> Text,
        f_code -> Text,
        f_pending -> Bool,
        f_role_mask -> Int8,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_obj_prom_task (f_id) {
        f_id -> Text,
        f_topic -> Text,
        f_oper -> Text,
        f_obj_id -> Text,
        f_version -> Int8,
        f_key -> Text,
        f_generation -> Int8,
        f_status -> Text,
        f_visible_at -> Timestamptz,
        f_retried_count -> Int8,
        f_lease -> Int8,
        f_error -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_page (f_id) {
        f_id -> Text,
        f_chapter_id -> Text,
        f_index -> Int4,
        f_total_unit_count -> Int4,
        f_translated_unit_count -> Int4,
        f_proofread_unit_count -> Int4,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_page_image (f_id) {
        f_id -> Text,
        f_version -> Int8,
        f_key -> Nullable<Text>,
        f_is_uploaded -> Nullable<Bool>,
        f_hash -> Nullable<Bytea>,
        f_ext -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_system_mail (f_id) {
        f_id -> Text,
        f_receiver_id -> Text,
        f_title -> Text,
        f_content -> Text,
        f_read -> Bool,
        f_created_at -> Timestamptz,
    }
}

diesel::table! {
    t_team (f_id) {
        f_id -> Text,
        f_name -> Text,
        f_description -> Nullable<Text>,
        f_workset_next_index -> Int4,
        f_deleted_at -> Nullable<Timestamptz>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_team_avatar (f_id) {
        f_id -> Text,
        f_version -> Int8,
        f_key -> Nullable<Text>,
        f_is_uploaded -> Nullable<Bool>,
        f_hash -> Nullable<Bytea>,
        f_ext -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_term (f_id) {
        f_id -> Text,
        f_termbase_id -> Text,
        f_source -> Text,
        f_targets -> Array<Nullable<Text>>,
        f_comment -> Nullable<Text>,
        f_creator_id -> Text,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_termbase (f_id) {
        f_id -> Text,
        f_team_id -> Nullable<Text>,
        f_comic_id -> Nullable<Text>,
        f_name -> Text,
        f_description -> Nullable<Text>,
        f_term_count -> Int4,
        f_creator_id -> Text,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_unit (f_id) {
        f_id -> Text,
        f_page_id -> Text,
        f_next_id -> Nullable<Text>,
        f_hidden_at -> Nullable<Timestamptz>,
        f_is_bubble -> Bool,
        f_is_proofread -> Bool,
        f_x_coord -> Float8,
        f_y_coord -> Float8,
        f_translated_text -> Nullable<Text>,
        f_last_translator_id -> Nullable<Text>,
        f_proofread_text -> Nullable<Text>,
        f_last_proofreader_id -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_user (f_id) {
        f_id -> Text,
        f_nickname -> Text,
        f_qid -> Text,
        f_is_sadmin -> Bool,
        f_password_hash -> Text,
        f_last_active_at -> Timestamptz,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_user_avatar (f_id) {
        f_id -> Text,
        f_version -> Int8,
        f_key -> Nullable<Text>,
        f_is_uploaded -> Nullable<Bool>,
        f_hash -> Nullable<Bytea>,
        f_ext -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_workset (f_id) {
        f_id -> Text,
        f_team_id -> Text,
        f_index -> Int4,
        f_name -> Text,
        f_description -> Nullable<Text>,
        f_comic_count -> Int4,
        f_comic_next_index -> Int4,
        f_deleted_at -> Nullable<Timestamptz>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::joinable!(t_announcement -> t_team (f_team_id));
diesel::joinable!(t_announcement -> t_user (f_user_id));
diesel::joinable!(t_assignment -> t_chapter (f_chapter_id));
diesel::joinable!(t_assignment -> t_user (f_user_id));
diesel::joinable!(t_assignment_invitation -> t_chapter (f_chapter_id));
diesel::joinable!(t_assignment_invitation -> t_user (f_inviter_id));
diesel::joinable!(t_chapter -> t_comic (f_comic_id));
diesel::joinable!(t_chapter -> t_user (f_creator_id));
diesel::joinable!(t_chapter_workflow_record -> t_chapter (f_chapter_id));
diesel::joinable!(t_comic -> t_user (f_creator_id));
diesel::joinable!(t_comic -> t_workset (f_workset_id));
diesel::joinable!(t_comic_archive -> t_team (f_team_id));
diesel::joinable!(t_comment -> t_team (f_team_id));
diesel::joinable!(t_comment -> t_user (f_user_id));
diesel::joinable!(t_member -> t_team (f_team_id));
diesel::joinable!(t_member -> t_user (f_user_id));
diesel::joinable!(t_member_invitation -> t_team (f_team_id));
diesel::joinable!(t_member_invitation -> t_user (f_inviter_id));
diesel::joinable!(t_page -> t_chapter (f_chapter_id));
diesel::joinable!(t_system_mail -> t_user (f_receiver_id));
diesel::joinable!(t_term -> t_termbase (f_termbase_id));
diesel::joinable!(t_term -> t_user (f_creator_id));
diesel::joinable!(t_termbase -> t_comic (f_comic_id));
diesel::joinable!(t_termbase -> t_team (f_team_id));
diesel::joinable!(t_termbase -> t_user (f_creator_id));
diesel::joinable!(t_unit -> t_page (f_page_id));
diesel::joinable!(t_workset -> t_team (f_team_id));

diesel::allow_tables_to_appear_in_same_query!(
    t_announcement,
    t_assignment,
    t_assignment_invitation,
    t_chapter,
    t_chapter_artwork,
    t_chapter_workflow_record,
    t_comic,
    t_comic_archive,
    t_comic_cover,
    t_comment,
    t_local_message,
    t_member,
    t_member_invitation,
    t_obj_prom_task,
    t_page,
    t_page_image,
    t_system_mail,
    t_team,
    t_team_avatar,
    t_term,
    t_termbase,
    t_unit,
    t_user,
    t_user_avatar,
    t_workset,
);
