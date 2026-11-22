// Two interfaces each give the class a member type `In`: javac refuses the name as ambiguous where it is used, and so
// does teq, where scalac reading the source takes the later parent's (docs/COMPATIBILITY.md).
object Main {
  def main(args: Array[String]): Unit = println(r.C.make())
}
