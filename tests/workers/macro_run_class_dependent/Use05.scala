// A call of the macro, its result's type the tag's member.
package own.use

import own.*

object Use05:
  val v: Int = Base.dep(5)(IntTag)
  def sum(xs: List[Int]): Int = xs.foldLeft(v)(_ + _)
