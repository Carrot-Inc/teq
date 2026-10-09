package gcob

import gco.*

def render[A](a: A)(using s: Show[A]): String = s.show(a)

@main def run(): Unit =
  println(Plain.sized(using 2).show("abcd"))
  println(Loud.sized(using 3).show("abcd"))
  println(render(7)(using Plain.given_Show_Int) + " " + render(8)(using Loud.given_Show_Int))
  val two = Scale(2)
  val ten = Scale(10)
  println(two.scaled(using 1).show(5L) + "," + ten.scaled(using 3).show(5L))
  println(render(true)(using two.given_Show_Boolean) + " " + render(false)(using ten.given_Show_Boolean))
  locally {
    import Loud.given
    given Int = 1
    println(render("xyz"))
  }
  locally {
    import ten.given
    given Int = 4
    println(render(6L) + " " + render(true))
  }
  val sub = Sub()
  println(sub.inside + " " + sub.lifted + " " + sub.viaThis + " " + new sub.Nested().deep)
  println(render(1)(using Tier.Low.tiered(using 1)) + " " + render(2)(using Tier.High.tiered(using 2)) + " " + render('c')(using Tier.High.given_Show_Char))
