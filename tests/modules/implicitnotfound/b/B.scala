package infb

import infa.{Codec, need, both}

object Use:
  val fine: Codec[Int] = summon[Codec[Int]]
  val missing: Codec[String] = summon[Codec[String]]
  val param = need[Boolean]
  val meet = both
