package tab

import taa.*

object Impl extends Api:
  def value: Int = 21

given Named[Int] with
  extension (a: Int) def named: String = "n" + a

object Use:
  def read(a: Api): Int = a.value
  def label(x: Int)(using n: Named[Int]): String = x.named
