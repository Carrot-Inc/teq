import fix.rdecl.*

object Twos:
  def run: Int = RdTraces().two + RdBox[Int](2).put("x").length
