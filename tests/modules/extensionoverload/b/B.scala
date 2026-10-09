package eob

import eoa.{Day, Hour}
import eoa.U.fmt

object B:
  def one(d: Day): String = d.fmt
  def all(ds: List[Day]): List[String] = ds.map(_.fmt)

@main def run(): Unit = println(B.all(List(Day(2))) :+ B.one(Day(4)) :+ Hour(5).fmt)
