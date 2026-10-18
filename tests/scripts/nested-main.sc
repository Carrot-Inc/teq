// No main of the script's object: a nested object's is none of its template's own members
// (Scala CLI's WrapperUtils.hasMainSignature), so the script's statements run, and there are none.
object Outer {
  object Inner {
    def main(args: Array[String]): Unit = println("inner")
  }
}
