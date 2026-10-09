package app

import util.Prelude.{*, given}

case class Account(owner: String, balance: Int)

def render[A](a: A)(using s: Show[A]): String = s.show(a)

def levelName(l: Level): String = l match
  case Level.Low => "L"
  case Level.High => "H"
  case Level.Custom(n) => "C" + n

@main def run(): Unit =
  // def, val, lazy val and a renamed def
  println(clamp(40, 0, defaultLimit))
  println(banner)
  println(clampInt(-5, 0, 10))

  // case class: type position, constructor, pattern match
  val t: Tw = cls("p-4", "flex")
  println(t.render)
  Tw(List("a", "b")) match
    case Tw(classes) => println(classes.length)
  println(classNames("a" -> true, "b" -> false, "c" -> true).render)
  println(tw)

  // enum
  val levels: List[Level] = List(Level.Low, Level.High, Level.Custom(3))
  println(levels.map(levelName))

  // opaque type with its extension methods
  val id: UserId = UserId(41)
  println(id.next.value)

  // extension methods: exported, and defined in the trait
  println("hey".shout)
  println(List(1, 2, 3).joined(n => s"#$n"))

  // members of the trait
  println(twice(21))
  println(limitTwice)

  // givens: using parameter and summon
  println(render(7))
  println(render(List(1, 2)))
  println(summon[Show[Level]].show(Level.Custom(9)))

  // package members
  val balance = Lens[Account, Int](_.balance, (a, b) => a.copy(balance = b))
  println(balance.modify(Account("ann", 10))(_ + 5))
  val iso: Iso[Int, String] = Iso(_.toString, _.length)
  println(iso.from(iso.to(12345)))
  val owner: Getter[Account, String] = _.owner
  println(owner(Account("zoe", 1)))

  // transitive exports, exports of a nested enum's cases
  println(describe(Desktop))
  println(List(Desktop, ViewVariant.Tablet, Mobile).sorted)
  println(IconSet.home)
  println(IconSet.homeTwice)
  val gear: IconDef = IconSet.icon("gear")
  println(gear)
  println(smallIcon("dot") match
    case IconDef(name, size) => name + "@" + size
  )

  // qualified access
  println(util.Prelude.clamp(5, 0, 3))
  println(util.Prelude.twice(4))
  println(util.Prelude.Tw(List("q")).render)
  println(util.Prelude.IconSet.user)
  println(util.Prelude.Level.High)
  println(sharedlib.Prelude.cls("z").render)
  val other: util.Prelude.IconDef = util.Prelude.IconDef("o", 1)
  println(other.size)

  // imports by name and through an exported object
  println(app.icons.iconLine)
  println(app.icons.styled.render)
  println(app.icons.four)
  println(app.icons.numbers)
