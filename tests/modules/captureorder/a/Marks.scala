package captureorder.marks

import scala.compiletime.summonInline

class First(val n: Int)
class Second(val n: Int)
class Third(val n: Int)
class Fourth(val n: Int)

// Each call is expanded after the typing: the given it finds joins the captures of the class of the block it
// stands in when the phase has run.
object Marks:
  inline def first: Int = summonInline[First].n
  inline def second: Int = summonInline[Second].n
  inline def third: Int = summonInline[Third].n
  inline def fourth: Int = summonInline[Fourth].n
