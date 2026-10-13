// A block-local object's member called as a partial function's block result keeps the block's match
// (no local type escapes the result), which scalac then warns of (E211) and does not synthesize.
val f: PartialFunction[Int, Int] = x =>
  object local:
    def answer(): Int = 42
  x match
    case 0 => local.answer()
