//> using scala 3.8.4
//> using file lib/Helper.scala
// `//> using file` includes a source, its path relative to this file, whatever the working
// directory: tests/run_interp.sh runs this from the repository's root. The other key is a comment.
object Main:
  def main(args: Array[String]): Unit = println(Helper.value)
