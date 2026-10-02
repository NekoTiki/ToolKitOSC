// Friendly "Adjective Animal" names for viewers who don't sign in with Discord, instead of every one
// of them showing up as "Guest". 150 adjectives x 150 animals = 22,500 combinations.
const ADJECTIVES = [
  'Agile', 'Amber', 'Ancient', 'Arctic', 'Astral', 'Autumn', 'Bashful', 'Blazing', 'Bold', 'Bouncy',
  'Brave', 'Breezy', 'Bright', 'Brisk', 'Bubbly', 'Calm', 'Candid', 'Cheeky', 'Cheerful', 'Chill',
  'Chirpy', 'Clever', 'Cloudy', 'Cosmic', 'Cozy', 'Crafty', 'Crimson', 'Crispy', 'Curious', 'Dapper',
  'Daring', 'Dashing', 'Dazzling', 'Dreamy', 'Dusky', 'Eager', 'Electric', 'Elegant', 'Emerald', 'Epic',
  'Fancy', 'Fearless', 'Feisty', 'Fiery', 'Fluffy', 'Frosty', 'Funky', 'Fuzzy', 'Gentle', 'Giddy',
  'Glowing', 'Golden', 'Goofy', 'Graceful', 'Grumpy', 'Happy', 'Hasty', 'Hazy', 'Heroic', 'Hidden',
  'Honest', 'Humble', 'Icy', 'Indigo', 'Jazzy', 'Jolly', 'Jumpy', 'Keen', 'Kind', 'Lazy',
  'Lively', 'Lucky', 'Lunar', 'Magic', 'Majestic', 'Mellow', 'Merry', 'Mighty', 'Minty', 'Misty',
  'Modest', 'Mossy', 'Nifty', 'Nimble', 'Noble', 'Nocturnal', 'Odd', 'Peppy', 'Perky', 'Playful',
  'Plucky', 'Polite', 'Proud', 'Quick', 'Quiet', 'Quirky', 'Radiant', 'Rapid', 'Rowdy', 'Royal',
  'Rusty', 'Salty', 'Sassy', 'Scarlet', 'Secret', 'Shiny', 'Shy', 'Silent', 'Silky', 'Silly',
  'Sleepy', 'Slick', 'Sly', 'Snappy', 'Sneaky', 'Snowy', 'Soft', 'Solar', 'Sparkly', 'Speedy',
  'Spicy', 'Spooky', 'Sprightly', 'Starry', 'Stealthy', 'Stormy', 'Sturdy', 'Sunny', 'Swift', 'Tidy',
  'Tiny', 'Toasty', 'Tranquil', 'Turbo', 'Twinkly', 'Velvet', 'Vivid', 'Wacky', 'Wandering', 'Warm',
  'Whimsical', 'Wild', 'Windy', 'Wise', 'Witty', 'Wobbly', 'Wondrous', 'Zany', 'Zealous', 'Zesty'
]

const ANIMALS = [
  'Aardvark', 'Albatross', 'Alpaca', 'Anteater', 'Armadillo', 'Axolotl', 'Badger', 'Bat', 'Beaver', 'Beetle',
  'Bison', 'Blobfish', 'Boar', 'Bobcat', 'Buffalo', 'Bumblebee', 'Butterfly', 'Camel', 'Capybara', 'Caracal',
  'Cardinal', 'Caribou', 'Catfish', 'Chameleon', 'Cheetah', 'Chickadee', 'Chinchilla', 'Chipmunk', 'Cobra', 'Cockatoo',
  'Condor', 'Corgi', 'Cougar', 'Coyote', 'Crab', 'Crane', 'Cricket', 'Crow', 'Cuttlefish', 'Dingo',
  'Dolphin', 'Donkey', 'Dormouse', 'Dragonfly', 'Duck', 'Eagle', 'Eel', 'Elk', 'Emu', 'Falcon',
  'Ferret', 'Finch', 'Firefly', 'Flamingo', 'Fox', 'Frog', 'Gazelle', 'Gecko', 'Gerbil', 'Gibbon',
  'Giraffe', 'Goat', 'Goose', 'Gopher', 'Gorilla', 'Grasshopper', 'Hamster', 'Hare', 'Hawk', 'Hedgehog',
  'Heron', 'Hippo', 'Hornet', 'Hummingbird', 'Husky', 'Hyena', 'Ibex', 'Iguana', 'Impala', 'Jackal',
  'Jaguar', 'Jellyfish', 'Kangaroo', 'Kingfisher', 'Kitten', 'Kiwi', 'Koala', 'Koi', 'Ladybug', 'Lemming',
  'Lemur', 'Leopard', 'Llama', 'Lobster', 'Lynx', 'Macaw', 'Magpie', 'Manatee', 'Mantis', 'Marmot',
  'Meerkat', 'Mink', 'Mole', 'Mongoose', 'Moose', 'Moth', 'Narwhal', 'Newt', 'Ocelot', 'Octopus',
  'Opossum', 'Orca', 'Ostrich', 'Otter', 'Owl', 'Panda', 'Pangolin', 'Panther', 'Parrot', 'Peacock',
  'Pelican', 'Penguin', 'Pigeon', 'Platypus', 'Porcupine', 'Possum', 'Puffin', 'Puma', 'Quail', 'Quokka',
  'Rabbit', 'Raccoon', 'Raven', 'Reindeer', 'Robin', 'Salamander', 'Seahorse', 'Seal', 'Shark', 'Sloth',
  'Snail', 'Sparrow', 'Squid', 'Squirrel', 'Starfish', 'Stingray', 'Tapir', 'Tiger', 'Toucan', 'Walrus'
]

// FNV-1a: a tiny, stable string hash, so a given seed maps to the same name on every machine.
function hash(seed: string): number {
  let h = 0x811c9dc5

  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i)
    h = Math.imul(h, 0x01000193)
  }

  return h >>> 0
}

// The same seed (a viewer's session id) always gives the same name, so a guest keeps their name
// across reconnects and reloads.
export function guestName(seed: string): string {
  const h = hash(seed)

  return `${ADJECTIVES[h % ADJECTIVES.length]} ${ANIMALS[Math.floor(h / ADJECTIVES.length) % ANIMALS.length]}`
}

export function randomGuestName(): string {
  return guestName(`${Math.random()}${Date.now()}`)
}
