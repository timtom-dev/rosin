#![forbid(unsafe_code)]

mod parser {
    // TODO check that each property is parsing correctly
}

mod style {
    mod properties {
        // TODO manually create stylesheets without parsing
    }
    mod selector_matching {
        use crate::prelude::*;
        use crate::tree::ArrayNode;
        use bumpalo::collections::Vec as BumpVec;

        #[test]
        fn basic_class() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static(".root { height: 100px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [] // 0
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, Some(100.0));
        }

        #[test]
        fn child() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static(".root .child { height: 100px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [ // 0
                    "child" [] // 1
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, None);
            assert_eq!(tree[1].style.height, Some(100.0));
        }

        #[test]
        fn direct_child() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static(".parent > .child { height: 100px; } .child { height: 200px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [ // 0
                    "parent" [ // 2
                        "child" [] // 3
                    ]
                    "parent" [ // 1
                        "wrap" [ // 4
                            "child" [] // 5
                        ]
                    ]
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, None);
            assert_eq!(tree[1].style.height, None);
            assert_eq!(tree[2].style.height, None);
            assert_eq!(tree[3].style.height, Some(100.0));
            assert_eq!(tree[4].style.height, None);
            assert_eq!(tree[5].style.height, Some(200.0));
        }

        #[test]
        fn specificity() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static(".root .child { height: 100px; } .child { height: 90px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [ // 0
                    "wrap" [ // 1
                        "child" [] // 2
                    ]
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, None);
            assert_eq!(tree[1].style.height, None);
            assert_eq!(tree[2].style.height, Some(100.0));
        }

        #[test]
        fn wildcard() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static("* { height: 100px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [ // 0
                    "parent" [ // 2
                        "child" [] // 3
                    ]
                    "parent" [ // 1
                        "wrap" [ // 4
                            "child" [] // 5
                        ]
                    ]
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, Some(100.0));
            assert_eq!(tree[1].style.height, Some(100.0));
            assert_eq!(tree[2].style.height, Some(100.0));
            assert_eq!(tree[3].style.height, Some(100.0));
            assert_eq!(tree[4].style.height, Some(100.0));
            assert_eq!(tree[5].style.height, Some(100.0));
        }

        #[test]
        fn wildcard_specificity() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static("* { height: 100px; } child { height: 1px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [ // 0
                    "parent" [ // 2
                        "child" [] // 3
                    ]
                    "parent" [ // 1
                        "wrap" [ // 4
                            "child" [] // 5
                        ]
                    ]
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, Some(100.0));
            assert_eq!(tree[1].style.height, Some(100.0));
            assert_eq!(tree[2].style.height, Some(100.0));
            assert_eq!(tree[3].style.height, Some(1.0));
            assert_eq!(tree[4].style.height, Some(100.0));
            assert_eq!(tree[5].style.height, Some(1.0));
        }

        #[test]
        fn wildcard_descendant() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static("* one { height: 100px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [ // 0
                    "parent" [ // 1
                        "one" [] // 3
                        "two" [] // 2
                    ]
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, None);
            assert_eq!(tree[1].style.height, None);
            assert_eq!(tree[2].style.height, None);
            assert_eq!(tree[3].style.height, Some(100.0));
        }

        #[test]
        fn non_matching() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static("selector { height: 100px; } random name { height: 100px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "root" [ // 0
                    "parent" [ // 1
                        "one" [] // 3
                        "two" [] // 2
                    ]
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, None);
            assert_eq!(tree[1].style.height, None);
            assert_eq!(tree[2].style.height, None);
            assert_eq!(tree[3].style.height, None);
        }

        #[test]
        fn semi_matching_chain() {
            let alloc = &Alloc::default();
            let stylesheet = Stylesheet::new_static("one two three { height: 100px; }");

            let mut tree: BumpVec<ArrayNode<()>> = ui! { alloc,
                "two" [ // 0
                    "three" [] // 1
                ]
            }
            .finish(&alloc)
            .unwrap();

            stylesheet.style(&mut tree);

            assert_eq!(tree[0].style.height, None);
            assert_eq!(tree[1].style.height, None);
        }
    }
}
