// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused, clippy::all)]
mod bridge {
    include!(concat!(env!("OUT_DIR"), "/bridge_mod.rs"));
}

#[cfg(test)]
mod tests {
    unsafe extern "C" {
        fn quent_demo_cpp_smoke() -> std::ffi::c_int;
        fn quent_demo_cpp_dynamic_try_into() -> std::ffi::c_int;
        fn quent_cpp_noop_fsm() -> bool;
        fn quent_demo_cpp_dynamic_values(output_dir: *const std::ffi::c_char) -> std::ffi::c_int;
    }

    #[test]
    fn compiles_links_and_runs_cpp_facade() {
        assert_eq!(unsafe { quent_demo_cpp_smoke() }, 0);
    }

    #[test]
    fn dynamic_try_into_retains_handle_on_mismatch() {
        assert_eq!(unsafe { quent_demo_cpp_dynamic_try_into() }, 0);
    }

    #[test]
    fn noop_skips_fsm_checks() {
        assert!(unsafe { quent_cpp_noop_fsm() });
    }

    #[test]
    fn preserves_dynamic_value_types_and_insertion_order() {
        let output_dir = std::env::temp_dir().join(format!(
            "quent-cpp-dynamic-values-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&output_dir).unwrap();
        let output_dir_c = std::ffi::CString::new(output_dir.to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { quent_demo_cpp_dynamic_values(output_dir_c.as_ptr()) },
            0,
        );

        let serialized = read_tree(&output_dir);
        std::fs::remove_dir_all(&output_dir).unwrap();

        let keys = [
            "null",
            "bool",
            "u8",
            "u16",
            "u32",
            "u64",
            "i8",
            "i16",
            "i32",
            "i64",
            "f32",
            "f64",
            "string",
            "structure",
            "first",
            "second",
            "u8_list",
            "u16_list",
            "u32_list",
            "u64_list",
            "i8_list",
            "i16_list",
            "i32_list",
            "i64_list",
            "f32_list",
            "f64_list",
            "string_list",
            "struct_list",
            "nested_list",
            "moved_to",
            "before_move",
            "moved_from",
            "after_move",
        ];
        let mut previous = 0;
        for key in keys {
            let position = serialized
                .find(&format!(r#""key":"{key}""#))
                .unwrap_or_else(|| panic!("missing dynamic attribute `{key}`: {serialized}"));
            assert!(
                position >= previous,
                "dynamic attribute `{key}` is out of order"
            );
            previous = position;
        }
        for value in [
            r#""value":null"#,
            r#""U8":1"#,
            r#""U8":8"#,
            r#""U16":16"#,
            r#""U32":32"#,
            r#""U64":64"#,
            r#""I8":-8"#,
            r#""I16":-16"#,
            r#""I32":-32"#,
            r#""I64":-64"#,
            r#""F32":32.5"#,
            r#""F64":64.5"#,
            r#""String":"value""#,
            r#""U8":[1,2]"#,
            r#""U16":[1,2]"#,
            r#""U32":[1,2]"#,
            r#""U64":[1,2]"#,
            r#""I8":[-1,2]"#,
            r#""I16":[-1,2]"#,
            r#""I32":[-1,2]"#,
            r#""I64":[-1,2]"#,
            r#""F32":[1.5,2.5]"#,
            r#""F64":[1.5,2.5]"#,
            r#""String":["first","second"]"#,
            r#""List":[{"U8":[1,2]},{"List":[{"String":["nested"]}]}]"#,
            r#""String":"retained""#,
            r#""String":"valid""#,
        ] {
            assert!(serialized.contains(value), "missing dynamic value {value}");
        }
        assert!(serialized.contains(
            r#""Struct":[[{"key":"name","value":{"String":"first"}}],[{"key":"name","value":{"String":"second"}}]]"#,
        ));
    }

    fn read_tree(path: &std::path::Path) -> String {
        let mut output = String::new();
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                output.push_str(&read_tree(&path));
            } else {
                output.push_str(&std::fs::read_to_string(path).unwrap());
            }
        }
        output
    }
}
