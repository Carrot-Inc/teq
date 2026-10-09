package gcb

import gca.Show

def render[A](a: A)(using s: Show[A]): String = s.show(a)

@main def run(): Unit =
  println(render(List(1, 2)))
  println(render(Option(3)))
  println(render(("x", 4)))
  println(render(List(Option(5), None)))
