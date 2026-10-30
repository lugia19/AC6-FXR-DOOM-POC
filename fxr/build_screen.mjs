// Builds the Doom screen: 160x100 camera-facing square particles, one node per pixel, each colored by three custom
// external values (R, G, B) that the fxr_doom DLL serves from Doom's 320x200 frames (every other pixel and row).
// One spawned effect stops creating particles at about 11.5k (child effects included), so the screen is two FXRs
// with 50 rows each, on two different bullets: TOP_ID on TURNER, BOTTOM_ID on SCUDDER.
// Usage: node build_screen.mjs <unpacked sfxbnd_commoneffects effect dir>
import {
  AttachMode,
  BasicNode,
  BillboardEx,
  BlendMode,
  ConstantProperty,
  ExternalValue1Modifier,
  FXR,
  Game,
  Keyframe,
  LinearProperty,
  OneTimeEmitter,
  ParticleAttributes,
  State,
  StaticNodeTransform,
} from '@cccode/fxr'
import path from 'node:path'

// Must match the DLL: source pixel (sx, sy) channel c (0 = R, 1 = G, 2 = B) is ID BASE + (sy * 320 + sx) * 4 + c,
// with the channel as 0..1.
const BASE = 1_000_000
const SOURCE_WIDTH = 320
const SOURCE_HEIGHT = 200
const extId = (sx, sy, c) => BASE + (sy * SOURCE_WIDTH + sx) * 4 + c

// Must match build.py.
const TOP_ID = 9_990_001
const BOTTOM_ID = 9_990_002
const WIDTH = 160
const HEIGHT = 100
const SCREEN_WIDTH = 16 // meters
const ELEVATION = 2     // meters from the bullet to the screen's bottom edge
// Where the TURNER bullet is relative to the SCUDDER bullet, in the SCUDDER bullet's frame (x right, y up,
// z = minus forward), so both halves share one anchor. Measured by logging the effects' spawn transforms with both
// guns fired standing still: the SCUDDER bullet starts 1.1 m further forward and 0.27 m higher.
const BOTTOM_ANCHOR = [0, -0.27, -1.107]

const pixelSize = SCREEN_WIDTH / WIDTH

// Multiplier for one channel: value 0 -> channel off, 1 -> channel full, linear in between.
function channel(id, c) {
  const factor = v => [0, 1, 2, 3].map(i => (i === c ? v : 1))
  return new ExternalValue1Modifier(id, new LinearProperty(false, [new Keyframe(0, factor(0)), new Keyframe(1, factor(1))]))
}

function pixelNode(x, y, anchor) {
  // Nearest source pixel to this pixel's center.
  const sx = Math.floor((x + 0.5) * SOURCE_WIDTH / WIDTH)
  const sy = Math.floor((y + 0.5) * SOURCE_HEIGHT / HEIGHT)
  const color = new ConstantProperty([1, 1, 1, 1])
  color.modifiers = [0, 1, 2].map(c => channel(extId(sx, sy, c), c))
  return new BasicNode([
    new StaticNodeTransform({ offset: [anchor[0], anchor[1] + ELEVATION, anchor[2]] }),
    // One particle that lives forever: its color modifiers are still evaluated every frame.
    new OneTimeEmitter(),
    new ParticleAttributes({ duration: -1, attachment: AttachMode.Parent }),
    new BillboardEx({
      texture: 1, // solid white square
      blendMode: BlendMode.Normal,
      width: pixelSize,
      height: pixelSize,
      // In units of the particle's size, in the billboard's plane: center the screen horizontally, row 0 on top.
      offsetX: x - (WIDTH - 1) / 2,
      offsetY: HEIGHT - 0.5 - y,
      color1: color,
    }),
  ])
}

async function save(id, fromRow, toRow, anchor) {
  const nodes = []
  for (let y = fromRow; y < toRow; y++) {
    for (let x = 0; x < WIDTH; x++) {
      nodes.push(pixelNode(x, y, anchor))
    }
  }
  const fxr = new FXR(id, [State.from(['ext(0) >= 1 then terminate'])], nodes)
  await fxr.saveAs(path.join(process.argv[2], fxr.name), Game.ArmoredCore6)
}

await save(TOP_ID, 0, HEIGHT / 2, [0, 0, 0])
await save(BOTTOM_ID, HEIGHT / 2, HEIGHT, BOTTOM_ANCHOR)
console.log(`Doom screen: ${WIDTH}x${HEIGHT} as ${TOP_ID} (top) and ${BOTTOM_ID} (bottom)`)
