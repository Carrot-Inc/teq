package dfa

// A dependent context function type in a signature, the TASTy inspector's shape over the
// library's own types: `fn`'s parameter names its contextual parameter.
object DepFn:
  def inspect[T](files: List[String])(fn: (ctx: String) ?=> List[Option[ctx.type]] => T): T =
    given s: String = files.mkString(",")
    fn(Nil)
