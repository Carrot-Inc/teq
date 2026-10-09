// A `Char` literal narrows to the `Byte` or `Short` expected when its value fits, as an `Int`
// literal does: a file signature spelled in characters.
@main def run(): Unit =
  val gif = Array[Byte]('G', 'I', 'F', '8')
  println(gif.mkString(","))
  val b: Byte = 'a'
  val s: Short = 'z'
  println(b + s)
