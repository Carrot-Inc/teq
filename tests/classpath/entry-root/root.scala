// An encoded `@main` of the root package beside an object's `main`: `java` launches its class `hello$minusworld`,
// which has no package, and `--main` chooses it by that name.
@main def `hello-world`(): Unit = println("hello-world at the root")

object Other:
  def main(args: Array[String]): Unit = println("other: " + args.mkString(","))
