//! Field schemas for the second half of the op set (opacity/text/structure/
//! document/token/recipe ops). Assembled with `group_a` by `super::op_fields`;
//! op names are unique across the two groups so match order is irrelevant.

use super::OpFieldSchema;

/// Field schemas for the ops handled by this group; `None` for any other name.
pub(super) fn op_fields(name: &str) -> Option<&'static [OpFieldSchema]> {
    match name {
        "set_opacity" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "opacity",
                    ty: "f64",
                    required: true,
                },
            ];
            Some(F)
        }
        "replace_text" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "spans",
                    ty: "[{text,fill?,font_weight?,italic?,…}]",
                    required: true,
                },
            ];
            Some(F)
        }
        "duplicate_node" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "new_id",
                    ty: "string",
                    required: true,
                },
            ];
            Some(F)
        }
        "duplicate_page" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "page",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "new_id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "id_suffix",
                    ty: "string",
                    required: true,
                },
            ];
            Some(F)
        }
        "group" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node_ids",
                    ty: "node-id[]",
                    required: true,
                },
                OpFieldSchema {
                    name: "group_id",
                    ty: "string",
                    required: true,
                },
            ];
            Some(F)
        }
        "ungroup" => {
            static F: &[OpFieldSchema] = &[OpFieldSchema {
                name: "group_id",
                ty: "node id",
                required: true,
            }];
            Some(F)
        }
        "reparent" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "new_parent",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "position",
                    ty: r#"{at:"last"} | {at:"first"} | {at:"index",index:N} | {at:"before",id:"<sibling-id>"} | {at:"after",id:"<sibling-id>"}"#,
                    required: false,
                },
            ];
            Some(F)
        }
        "align_nodes" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node_ids",
                    ty: "node-id[]",
                    required: true,
                },
                OpFieldSchema {
                    name: "align",
                    ty: "enum: left|hcenter|right|top|vcenter|bottom",
                    required: true,
                },
                OpFieldSchema {
                    name: "anchor",
                    ty: "string",
                    required: false,
                },
            ];
            Some(F)
        }
        "set_text_overflow" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node_id",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "overflow",
                    ty: "enum: fit|clip|visible",
                    required: true,
                },
            ];
            Some(F)
        }
        "add_page" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "w",
                    ty: "px",
                    required: true,
                },
                OpFieldSchema {
                    name: "h",
                    ty: "px",
                    required: true,
                },
                OpFieldSchema {
                    name: "background",
                    ty: "token ref",
                    required: false,
                },
                OpFieldSchema {
                    name: "index",
                    ty: "i64",
                    required: false,
                },
            ];
            Some(F)
        }
        "delete_page" => {
            static F: &[OpFieldSchema] = &[OpFieldSchema {
                name: "page",
                ty: "node id",
                required: true,
            }];
            Some(F)
        }
        "reorder_pages" => {
            static F: &[OpFieldSchema] = &[OpFieldSchema {
                name: "order",
                ty: "node-id[]",
                required: true,
            }];
            Some(F)
        }
        "add_asset" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "kind",
                    ty: "enum: image|svg|font",
                    required: true,
                },
                OpFieldSchema {
                    name: "src",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "sha256",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "producer_kind",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "producer_source",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_prompt",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_model",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_provider",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_seed",
                    ty: "integer",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_generation_date",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_license",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_source_rights",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_safety_status",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "ai_reuse_policy",
                    ty: "string",
                    required: false,
                },
            ];
            Some(F)
        }
        "set_asset" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node_id",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "asset_id",
                    ty: "string",
                    required: true,
                },
            ];
            Some(F)
        }
        "distribute_nodes" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node_ids",
                    ty: "node-id[]",
                    required: true,
                },
                OpFieldSchema {
                    name: "axis",
                    ty: "enum: horizontal|vertical",
                    required: true,
                },
            ];
            Some(F)
        }
        "create_token" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "type",
                    ty: "enum: color|dimension|number|fontFamily|fontWeight",
                    required: true,
                },
                OpFieldSchema {
                    name: "value",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "set",
                    ty: "string",
                    required: false,
                },
            ];
            Some(F)
        }
        "update_token_value" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "value",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "set",
                    ty: "string",
                    required: false,
                },
            ];
            Some(F)
        }
        "set_style_property" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "style_id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "property",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "value",
                    ty: "token ref",
                    required: true,
                },
            ];
            Some(F)
        }
        "set_text_direction" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "direction",
                    ty: "enum: ltr|rtl",
                    required: true,
                },
            ];
            Some(F)
        }
        "find_replace_text" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "find",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "replace",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "node",
                    ty: "node id",
                    required: false,
                },
            ];
            Some(F)
        }
        "set_page_size" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "page",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "w",
                    ty: "px",
                    required: true,
                },
                OpFieldSchema {
                    name: "h",
                    ty: "px",
                    required: true,
                },
            ];
            Some(F)
        }
        "align_to_edge" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "node",
                    ty: "node id",
                    required: true,
                },
                OpFieldSchema {
                    name: "edge",
                    ty: "enum: left|right|top|bottom|hcenter|vcenter",
                    required: true,
                },
                OpFieldSchema {
                    name: "margin",
                    ty: "f64",
                    required: false,
                },
            ];
            Some(F)
        }
        "create_recipe" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "kind",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "seed",
                    ty: "i64",
                    required: false,
                },
                OpFieldSchema {
                    name: "generator",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "bounds",
                    ty: "node id",
                    required: false,
                },
                OpFieldSchema {
                    name: "detached",
                    ty: "bool",
                    required: false,
                },
            ];
            Some(F)
        }
        "update_recipe" => {
            static F: &[OpFieldSchema] = &[
                OpFieldSchema {
                    name: "id",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "kind",
                    ty: "string",
                    required: true,
                },
                OpFieldSchema {
                    name: "seed",
                    ty: "i64",
                    required: false,
                },
                OpFieldSchema {
                    name: "generator",
                    ty: "string",
                    required: false,
                },
                OpFieldSchema {
                    name: "bounds",
                    ty: "node id",
                    required: false,
                },
                OpFieldSchema {
                    name: "detached",
                    ty: "bool",
                    required: false,
                },
            ];
            Some(F)
        }
        "delete_recipe" => {
            static F: &[OpFieldSchema] = &[OpFieldSchema {
                name: "id",
                ty: "string",
                required: true,
            }];
            Some(F)
        }
        _ => None,
    }
}
