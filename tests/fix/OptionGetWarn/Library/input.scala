/*
rule = OptionGetWarn
*/
package fix

object Library:
  def uses(xs: List[Int], m: Map[Int, Int]): Unit =
    // The receiver a member of scala-library's, called here.
    xs.headOption.get // assert: OptionGetWarn
    m.get(1).get // assert: OptionGetWarn
    sys.env.get("HOME").get // assert: OptionGetWarn
    scala.util.Try(1).toOption.get // assert: OptionGetWarn
    xs.find(_ > 0).get // assert: OptionGetWarn
    Option(1).get // assert: OptionGetWarn
    Some(1).get
    Option.empty[Int].get // assert: OptionGetWarn
    xs.headOption.map(_ + 1).get // assert: OptionGetWarn
    // A local inferred from such a member, and one declared.
    val fromStd = xs.headOption
    fromStd.get // assert: OptionGetWarn
    val o: Option[Int] = xs.lastOption
    o.get // assert: OptionGetWarn
    ()
