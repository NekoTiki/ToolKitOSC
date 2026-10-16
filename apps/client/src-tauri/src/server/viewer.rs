//! Who sent a viewer command, worked out from the client list the server sends, the same way the
//! webview's `describeViewer` (useClientsDb.ts) does: the viewer key that groups and bans use,
//! their stable IP, and the name shown for them.
use std::net::IpAddr;

use serde_json::Value;

// The share page's guest names, copied from packages/shared-ui/src/utils/guestName.ts. Change both.
const ADJECTIVES: [&str; 150] = [
    "Agile", "Amber", "Ancient", "Arctic", "Astral", "Autumn", "Bashful", "Blazing", "Bold", "Bouncy",
    "Brave", "Breezy", "Bright", "Brisk", "Bubbly", "Calm", "Candid", "Cheeky", "Cheerful", "Chill",
    "Chirpy", "Clever", "Cloudy", "Cosmic", "Cozy", "Crafty", "Crimson", "Crispy", "Curious", "Dapper",
    "Daring", "Dashing", "Dazzling", "Dreamy", "Dusky", "Eager", "Electric", "Elegant", "Emerald", "Epic",
    "Fancy", "Fearless", "Feisty", "Fiery", "Fluffy", "Frosty", "Funky", "Fuzzy", "Gentle", "Giddy",
    "Glowing", "Golden", "Goofy", "Graceful", "Grumpy", "Happy", "Hasty", "Hazy", "Heroic", "Hidden",
    "Honest", "Humble", "Icy", "Indigo", "Jazzy", "Jolly", "Jumpy", "Keen", "Kind", "Lazy",
    "Lively", "Lucky", "Lunar", "Magic", "Majestic", "Mellow", "Merry", "Mighty", "Minty", "Misty",
    "Modest", "Mossy", "Nifty", "Nimble", "Noble", "Nocturnal", "Odd", "Peppy", "Perky", "Playful",
    "Plucky", "Polite", "Proud", "Quick", "Quiet", "Quirky", "Radiant", "Rapid", "Rowdy", "Royal",
    "Rusty", "Salty", "Sassy", "Scarlet", "Secret", "Shiny", "Shy", "Silent", "Silky", "Silly",
    "Sleepy", "Slick", "Sly", "Snappy", "Sneaky", "Snowy", "Soft", "Solar", "Sparkly", "Speedy",
    "Spicy", "Spooky", "Sprightly", "Starry", "Stealthy", "Stormy", "Sturdy", "Sunny", "Swift", "Tidy",
    "Tiny", "Toasty", "Tranquil", "Turbo", "Twinkly", "Velvet", "Vivid", "Wacky", "Wandering", "Warm",
    "Whimsical", "Wild", "Windy", "Wise", "Witty", "Wobbly", "Wondrous", "Zany", "Zealous", "Zesty"
];

const ANIMALS: [&str; 150] = [
    "Aardvark", "Albatross", "Alpaca", "Anteater", "Armadillo", "Axolotl", "Badger", "Bat", "Beaver", "Beetle",
    "Bison", "Blobfish", "Boar", "Bobcat", "Buffalo", "Bumblebee", "Butterfly", "Camel", "Capybara", "Caracal",
    "Cardinal", "Caribou", "Catfish", "Chameleon", "Cheetah", "Chickadee", "Chinchilla", "Chipmunk", "Cobra", "Cockatoo",
    "Condor", "Corgi", "Cougar", "Coyote", "Crab", "Crane", "Cricket", "Crow", "Cuttlefish", "Dingo",
    "Dolphin", "Donkey", "Dormouse", "Dragonfly", "Duck", "Eagle", "Eel", "Elk", "Emu", "Falcon",
    "Ferret", "Finch", "Firefly", "Flamingo", "Fox", "Frog", "Gazelle", "Gecko", "Gerbil", "Gibbon",
    "Giraffe", "Goat", "Goose", "Gopher", "Gorilla", "Grasshopper", "Hamster", "Hare", "Hawk", "Hedgehog",
    "Heron", "Hippo", "Hornet", "Hummingbird", "Husky", "Hyena", "Ibex", "Iguana", "Impala", "Jackal",
    "Jaguar", "Jellyfish", "Kangaroo", "Kingfisher", "Kitten", "Kiwi", "Koala", "Koi", "Ladybug", "Lemming",
    "Lemur", "Leopard", "Llama", "Lobster", "Lynx", "Macaw", "Magpie", "Manatee", "Mantis", "Marmot",
    "Meerkat", "Mink", "Mole", "Mongoose", "Moose", "Moth", "Narwhal", "Newt", "Ocelot", "Octopus",
    "Opossum", "Orca", "Ostrich", "Otter", "Owl", "Panda", "Pangolin", "Panther", "Parrot", "Peacock",
    "Pelican", "Penguin", "Pigeon", "Platypus", "Porcupine", "Possum", "Puffin", "Puma", "Quail", "Quokka",
    "Rabbit", "Raccoon", "Raven", "Reindeer", "Robin", "Salamander", "Seahorse", "Seal", "Shark", "Sloth",
    "Snail", "Sparrow", "Squid", "Squirrel", "Starfish", "Stingray", "Tapir", "Tiger", "Toucan", "Walrus"
];

/// FNV-1a over UTF-16 units, like the TypeScript `charCodeAt` loop.
fn hash(seed: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for unit in seed.encode_utf16() {
        h ^= unit as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

/// A guest's "Adjective Animal" name, the same for a given session id everywhere.
pub fn guest_name(seed: &str) -> String {
    let h = hash(seed) as usize;
    format!("{} {}", ADJECTIVES[h % ADJECTIVES.len()], ANIMALS[(h / ADJECTIVES.len()) % ANIMALS.len()])
}

/// The address bans match on: an IPv4 address as is, an IPv6 one cut to its first 4 groups (a
/// home network's prefix), like `getStableIp`.
pub fn stable_ip(ip: &str) -> String {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V6(v6)) => v6.segments()[..4].iter().map(|part| format!("{part:x}")).collect::<Vec<_>>().join(":"),
        Ok(IpAddr::V4(v4)) => v4.to_string(),
        Err(_) => ip.to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct Viewer {
    /// `discord:<id>` when signed in, otherwise `guest:<peerId>`.
    pub key: String,
    pub discord_id: Option<String>,
    pub ip: String,
    pub display_name: String,
    /// The client check (see `passes_client_check`) wants a name.
    pub has_name: bool
}

fn text<'a>(value: &'a Value, path: &str) -> Option<&'a str> {
    value.pointer(path).and_then(Value::as_str).filter(|text| !text.is_empty())
}

/// One entry of the server's `client-list`.
pub fn describe(client: &Value) -> Option<Viewer> {
    let peer_id = text(client, "/peerId")?.to_owned();
    let discord_id = text(client, "/user/discord/id").map(str::to_owned);
    let user_name = text(client, "/user/userDefinedDisplayName");
    let guest = user_name.map(str::to_owned).unwrap_or_else(|| guest_name(&peer_id));
    let display_name = text(client, "/user/discord/name").or(text(client, "/user/username")).map(str::to_owned).unwrap_or(guest);

    Some(Viewer {
        key: match &discord_id {
            Some(id) => format!("discord:{id}"),
            None => format!("guest:{peer_id}")
        },
        has_name: user_name.is_some() || discord_id.is_some(),
        discord_id,
        ip: stable_ip(client.get("ip").and_then(Value::as_str).unwrap_or("unknown")),
        display_name
    })
}

/// The host's "Who can use your controls" setting (useClientType.ts).
pub fn passes_client_check(client_type: &str, viewer: &Viewer) -> bool {
    match client_type {
        "discord" => viewer.discord_id.is_some(),
        "username" => viewer.has_name,
        _ => true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_names_match_the_share_page() {
        // From the TypeScript guestName().
        assert_eq!(guest_name("abc"), "Toasty Hawk");
        assert_eq!(guest_name("7f3c2a90-peer"), "Polite Raccoon");
    }

    #[test]
    fn stable_ip_keeps_the_prefix() {
        assert_eq!(stable_ip("2a01:cb00:0abc:1200:1:2:3:4"), "2a01:cb00:abc:1200");
        assert_eq!(stable_ip("192.168.1.20"), "192.168.1.20");
        assert_eq!(stable_ip("unknown"), "unknown");
    }

    #[test]
    fn describe_prefers_discord() {
        let viewer = describe(&serde_json::json!({
            "peerId": "p1", "ip": "10.0.0.1", "user": { "discord": { "id": "42", "name": "Sam", "avatar": "" } }
        }))
        .unwrap();
        assert_eq!(viewer.key, "discord:42");
        assert_eq!(viewer.display_name, "Sam");
        assert!(passes_client_check("discord", &viewer));

        let guest = describe(&serde_json::json!({ "peerId": "p2", "ip": "10.0.0.2", "user": null })).unwrap();
        assert_eq!(guest.key, "guest:p2");
        assert!(!passes_client_check("username", &guest));
    }
}
