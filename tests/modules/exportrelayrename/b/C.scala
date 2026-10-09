package rrb

object C:
  export rra.B.{renamed, start}

@main def run(): Unit =
  println(C.renamed(41))
  println(C.start)
