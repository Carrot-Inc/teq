/* A directory is the sources under it, nested ones included and hidden ones left out. A block
   comment before the directive is part of the header. */
//> using file lib/deep
object Main:
  def main(args: Array[String]): Unit = println(Other.value + ", " + Inner.value)
