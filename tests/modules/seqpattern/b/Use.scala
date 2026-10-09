package spb

import spa.*

@main def run(): Unit =
  println(SeqPats.exact(List("a", "b")))
  println(SeqPats.named(List(1, 2, 3)))
  println(SeqPats.anon(Seq(4, 5)))
  println(SeqPats.anon(List(1, 2, 3)))
  println(SeqPats.nested(List(Seq(1), Seq(2, 3))))
  println(SeqPats.rep(Rep("ab", 1, 2, 3)))
  println(SeqPats.rep(Rep("abc")))
  println(SeqPats.words("x y"))
  println(SeqPats.node(new Node("n", 1, 2, 3)))
  println(SeqPats.onAny(List(3, 4)))
  println(SeqPats.onAny(Vector()))
  println(SeqPats.arr(Array(5, 6)))
  println(SeqPats.startsWithOne(List(1, 2, 3)))
  println(SeqPats.startsWithOne(List(2)))
  println(SeqPats.restOf(List(7, 8, 9)))
  Rep("z", 4, 5) match
    case Rep(_, first, _*) => println(first)
    case _ => println("none")
