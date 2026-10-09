// expect: cannot export nope: it is not a member of Source
// expect: cannot resolve export: missing not found
// expect: cannot resolve export: Inner not found
// expect: cannot resolve export: value is not an object or a package
// expect: an export needs a qualifier
// expect: cyclic export of Cycle
// expect: no given instance of type Tag[Int]
// expect: not found: secret
// expect: not found: hiddenOne
// expect: not found: notReexported
// expect: value nope is not a member of Api

import Api.*

trait Tag[A]:
  def tag: String

object Source:
  def visible: Int = 1
  def hiddenOne: Int = 2
  def notReexported: Int = 3
  private def secret: Int = 4
  given Tag[Int] with
    def tag: String = "int"

object Api:
  val value: Int = 1
  export Source.{nope}
  export missing.Thing.*
  export Source.Inner.*
  export value.*
  export visible
  export Source.{hiddenOne as _, notReexported as renamed, *}

object CycleA:
  export CycleB.*

object CycleB:
  export CycleA.*

def tagOf[A](using t: Tag[A]): String = t.tag

@main def run(): Unit =
  println(visible + renamed)
  println(secret)
  println(hiddenOne)
  println(notReexported)
  println(tagOf[Int])
  println(Api.nope)
