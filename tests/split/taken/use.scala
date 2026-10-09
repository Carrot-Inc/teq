package use

import parts.{*, given}

class Square(side: Int) extends Base("square") with Shape:
  def area: Int = side * side

enum Wrapped(val kind: Kind):
  case First extends Wrapped(Kind.Small)
  case Second extends Wrapped(Kind.Sized(7))

object Uses:
  def kindName(k: Kind): String = k match
    case Kind.Small => "small"
    case Kind.Large => "large"
    case Kind.Sized(n) => s"sized $n"

  def report(): Unit =
    val s = new Square(3)
    println(s.greet)
    println(s.describe)
    println(s.isInstanceOf[Shape])
    println(kindName(Kind.Small))
    println(kindName(summon[Kind]))
    println(kindName(Kind.Sized(twice(2))))
    println(kindName(Wrapped.First.kind))
    println(kindName(Wrapped.Second.kind))
    println(Registry.total)
    println(Registry.total)
    println(top)
    counter += 1
    println(counter)
    println(lazyTop)
    println(lazyTop)
