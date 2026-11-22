// `permits` and `sealed` are javac's contextual keywords: outside a class's header they are names, a field's and a
// method's here, as javac compiles them (docs/COMPATIBILITY.md: scalac refuses them reading the source); `sealed`
// is Scala's keyword, written in backquotes.
object Main {
  def main(args: Array[String]): Unit = println(s"${Names.`sealed`(2)} ${Names.permits}")
}
