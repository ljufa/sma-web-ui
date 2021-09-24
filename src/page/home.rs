use seed::{prelude::*, *};
use crate::Urls;

pub fn view<Ms>(base_url: &Url) -> Node<Ms> {
    section![C!["hero", "is-medium", "ml-6"],
        div![C!["hero-body"],
            h1![C!["title", "is-size-1"],
                "SMA",
            ],
            a![attrs!{At::Href => "https://sma.ljubojevic.freemyip.com/"},
                h2![C!["subtitle", "is-size-3"],
                    "SMA"
                ]
            ],
            a![C!["button", "is-primary", "mt-5", "is-size-5"], attrs!{At::Href => Urls::new(base_url).settings()},
                strong!["Go SMA"],
            ],
        ]
    ]
}
