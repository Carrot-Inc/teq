// Identity hashes a macro bakes into the code it expands to: an object the macro's own file
// keeps, and an object it makes from the expansion's position. teq's are their contents',
// so a watch session's retype of this file, which expands
// the macros again over the objects its interpreter kept, writes what a fresh build writes.
object Use:
  val label = "hash"
  val kept: Int = keptHash
  val site: Int = siteHash

@main def run(): Unit = println(s"${Use.label} ${Use.kept != 0} ${Use.site != 0}")
