package app

import scala.language.dynamics

import lib.{Reads, Record}

class Local(tag: String) extends Record(tag):
  def own: String = this.inherited + "|" + this.applied(1)

@main def run(): Unit =
  val r = new Record("r")
  println(Reads.field(r))
  println(Reads.call(r, 3))
  println(Reads.named(r))
  Reads.assign(r, 7)
  println(Reads.chained(r))
  println(r.direct)
  println(r.direct(1, 2))
  r.late = "v"
  println(new Local("l").own)
