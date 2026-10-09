package xpb

import xpa.Api.twice

object Use:
  val h: Int = xpa.Api.helper(1)
  val l: Int = xpa.Api.limit
  val t: xpa.Api.Thing = xpa.Api.Thing.one
  val p: xpa.Api.Pairs = List((1, 2))
  val w: Int = 3.twice
  val s: String = xpa.Api.Tools.hammer
