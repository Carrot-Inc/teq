package infb

import infa.{Codec, need, both, Service}

object Use:
  val fine: Codec[Int] = summon[Codec[Int]]
  val missing: Codec[String] = summon[Codec[String]]
  val param = need[Boolean]
  val meet = both
  val owner = new Service[Char].use
