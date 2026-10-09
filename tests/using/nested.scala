/* A directive inside a nested block comment is a comment, as scalac reads it: this comment
  /* holds another */
  //> using file not-a-real-dependency.scala
  and ends here. The directive after it includes its file. */
//> using file lib/Helper.scala
object Main:
  def main(args: Array[String]): Unit = println(Helper.value)
