package infb

import infa.Codec

object Use:
  val fine: Codec[Int] = summon[Codec[Int]]
  val missing: Codec[String] = summon[Codec[String]]
