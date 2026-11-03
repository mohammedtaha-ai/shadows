use crate::{Address, Member};

pub(super) fn addresses(body: &str, members: &[Member], speaker: Option<&str>) -> Vec<Address> {
    let tokens: Vec<_> = body
        .split(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '@' | '-' | '_')))
        .filter_map(|token| token.strip_prefix('@'))
        .collect();
    let mut addressed: Vec<_> = members
        .iter()
        .filter(|member| Some(member.slug.as_str()) != speaker)
        .filter(|member| {
            tokens
                .iter()
                .any(|token| token.eq_ignore_ascii_case(&member.slug))
        })
        .map(|member| Address::Agent {
            slug: member.slug.clone(),
        })
        .collect();
    let to_person = tokens
        .iter()
        .any(|token| token.eq_ignore_ascii_case("mohammed"));
    if speaker.is_none() && addressed.is_empty() && !to_person {
        addressed.extend(members.iter().map(|member| Address::Agent {
            slug: member.slug.clone(),
        }));
    }
    if to_person {
        addressed.push(Address::Person);
    }
    addressed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(slug: &str) -> Member {
        Member {
            slug: slug.into(),
            name: slug.into(),
            thread_id: core_threads::ThreadId::generate(),
        }
    }

    #[test]
    fn a_reply_to_mohammed_only_wakes_no_members() {
        let members = [member("alpha"), member("beta")];
        assert_eq!(
            addresses("@alpha @moHAMMed @beta-more", &members, Some("alpha")),
            [Address::Person],
        );
        assert!(addresses("ordinary reply", &members, Some("alpha")).is_empty());
    }

    #[test]
    fn no_mention_reaches_every_member() {
        let members = [member("alpha"), member("beta")];
        let all = [
            Address::Agent {
                slug: "alpha".into(),
            },
            Address::Agent {
                slug: "beta".into(),
            },
        ];
        assert_eq!(addresses("ordinary message", &members, None), all);
        assert_eq!(addresses("@outsider", &members, None), all);
    }

    #[test]
    fn a_person_address_alone_does_not_wake_members() {
        let members = [member("alpha"), member("beta")];
        assert_eq!(addresses("@mohammed", &members, None), [Address::Person]);
    }
}
