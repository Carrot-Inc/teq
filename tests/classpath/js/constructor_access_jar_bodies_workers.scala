// jars: ctorbodies-lib
// targets: js interp jvm
// teq: --threads 8
// constructor_access_jar_bodies at eight workers, the library's class extended by a class of this program and its
// bodies reached from a lambda: the same bodies read from the jar, asked nothing.
class Bench(k: Int) extends paper.Mill(k):
  def all: List[Int] =
    val inner = new Inner
    List(() => stamp(2), () => stampOf("abc"), () => kept(), () => keptDefault(), () => inlineWax(4), () => lambda(), () => inner.stamp(5)).map(_())
// A generic class of the library named bare, with an empty body: the creation its public constructor serves, and
// inside a subclass the protected one's.
class RackSub extends paper.Rack[Int]:
  def inner = new paper.Rack {}
@main def run(): Unit =
  println((new paper.Tray {}).v + new RackSub().inner.v)
  println(new Bench(1).all.mkString(" "))
  println(List(0, 1, 2).map(k => new Bench(k).stamp(1)).mkString(" "))
  println(paper.Mill.default.inlineWax(9))
