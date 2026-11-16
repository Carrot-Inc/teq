// A trait of the program's own named `App`, in another file of the empty package.
trait App { def tag: String = "the program's own"; def main(args: Array[String]): Unit = println(tag) }
