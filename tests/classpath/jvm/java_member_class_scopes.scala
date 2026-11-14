// jars: scala-library
// std: scala-library
// targets: jvm
// A Java class's static member class is a member of the class's companion alone, as scalac enters it: below a
// Java type with a static member `Kind` (or `Double`) a class's own, inherited or imported `Kind` is what the
// name means in every position. Each package is one use; the lines are the ones scalac prints.

// A self type that names the two.
package selfmix {
class Holder { class Kind(val n: Int) }
trait Marks { self: java.nio.file.WatchEvent[String] & Holder =>
  def mine: Kind = new Kind(7)
}
class Ev extends Holder with java.nio.file.WatchEvent[String] with Marks:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
def run(): Unit =
  println(new Ev().mine.n)
}

// A type pattern.
package pattern {
class Holder { class Kind(val n: Int) }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def test(x: Any): String = x match
    case k: Kind => "kind " + k.n
    case _ => "other"
  def mk: Any = new Kind(5)
def run(): Unit =
  val e = new Ev()
  println(e.test(e.mk))
  println(e.test("s"))
}

// A class that extends the name.
package subclass {
class Holder { class Kind(val n: Int) }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  class Finer(m: Int) extends Kind(m + 1)
  def mk: Kind = new Finer(1)
def run(): Unit =
  println(new Ev().mk.n)
}

// The class below holds a type alias of the name.
package alias {
class Holder { type Kind = Int }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mine: Kind = 7
def run(): Unit =
  println(new Ev().mine + 1)
}

// The name on an instance's path.
package path {
class Holder { class Kind(val n: Int) }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
def run(): Unit =
  val e = new Ev()
  val k: e.Kind = new e.Kind(9)
  println(k.n)
}

// An object below the two.
package module {
class Holder { class Kind(val n: Int) }
object Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mine: Kind = new Kind(7)
def run(): Unit =
  val k: Ev.Kind = Ev.mine
  println(k.n)
}

// A case class of the name: its constructor, extractor and copy.
package caseclass {
class Holder { case class Kind(n: Int) }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mk: Kind = Kind(7)
  def un(x: Any): Int = x match
    case Kind(n) => n
    case _ => -1
  def cp: Kind = mk.copy(n = 8)
def run(): Unit =
  val e = new Ev()
  println(e.un(e.mk) + e.cp.n)
}

// An export of the instance's members.
package exported {
class Holder { class Kind(val n: Int) }
class User { val e = new Ev(); export e.* }
class Ev extends Holder with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mk: Kind = new Kind(7)
def run(): Unit =
  val u = new User()
  println(u.mk.n)
  val k: u.e.Kind = u.mk
  println(k.n)
}

// A Java instance's members exported: `Double` stays `scala.Double`.
package wrapped {
class Wrap:
  val p = new java.awt.Point(3, 4)
  export p.*
  def half: Double = getX() / 2
  def fl: Float = 1.5f
def run(): Unit = println(new Wrap().half + new Wrap().fl)
}

// A class below a Java class declares a class of the static member class's name.
package declared {
class Mine(a: Int) extends java.awt.Point(a, a):
  class Double(val v: Int)
  def mk: Double = new Double(3)
def run(): Unit = println(new Mine(1).mk.v)
}

// A Java instance's members imported: the static member class is not among them.
package imported {
def run(): Unit =
  val p = new java.awt.Point(1, 2)
  import p.*
  val d: Double = 1.5
  println(d + getX())
}

// A later base holds a type alias of the name.
package typealias {
trait Named { type Kind = String }
class Ev extends Named with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def mine: Kind = "alias"
def run(): Unit =
  println(new Ev().mine)
}

// A later base holds an abstract type member of the name.
package abstracttype {
trait Named { type Kind; def first: Kind }
abstract class Ev extends Named with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def again: Kind = first
def run(): Unit =
  val e = new Ev { type Kind = Int; def first: Int = 4 }
  println(e.again)
}

// A later base holds an opaque type of the name.
package opaquetype {
trait Named:
  opaque type Kind = Int
  def first: Kind = 3
object Ev extends Named with java.nio.file.WatchEvent[String]:
  def kind(): java.nio.file.WatchEvent.Kind[String] = null
  def count(): Int = 1
  def context(): String = "c"
  def again: Kind = first
def run(): Unit =
  println(Ev.again)
}

@main def run(): Unit =
  selfmix.run()
  pattern.run()
  subclass.run()
  alias.run()
  path.run()
  module.run()
  caseclass.run()
  exported.run()
  wrapped.run()
  declared.run()
  imported.run()
  typealias.run()
  abstracttype.run()
  opaquetype.run()
