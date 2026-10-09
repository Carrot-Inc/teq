package aob

import aoa.*

class Both extends Impl with Plus with Loud

object Use:
  val n: Int = new Both().f
  val s: String = new Both().name
  val anon: Int = (new Impl with Plus).f
