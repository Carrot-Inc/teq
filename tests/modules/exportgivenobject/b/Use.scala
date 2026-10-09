package gob

import goa.{B, C, Tag}
import B.given
import C.given

@main def run(): Unit = println(summon[Tag].value)
