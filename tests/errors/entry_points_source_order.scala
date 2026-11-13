// command: build
// expect: several entry points: Kind.main, entry_points_source_order$package.main, Later.main, Last.main, Sort.main; choose one with --main
// A `main` inherited by an object written after its enum, a package's own, an object's own, an object's
// inherited one, and a second enum's object at the end: the entry points are named in the order they are
// written, an enum's object where the object is, and the error stands at the first of them.

enum Kind { case Plain }
enum Sort { case Fine }

trait Runs:
  def main(args: Array[String]): Unit = println("inherited")

object Kind extends Runs

def main(args: Array[String]): Unit = println("package")

object Later:
  def main(args: Array[String]): Unit = println("object")

object Last extends Runs

object Sort extends Runs
