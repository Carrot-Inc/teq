package meridian.core.amount

/** A decimal quantity with four places, kept exact as a count of ten-thousandths; the shape of a
  * money or measurement type without a floating-point number anywhere near the output. */
object Amounts:
  opaque type Amount = Long

  object Amount:
    def zero: Amount = 0L
    private val Scale = 10000L
    def ofUnits(units: Long): Amount = units * Scale
    def ofMinor(minor: Long): Amount = minor
    def parse(text: String): Either[String, Amount] =
      val negative = text.startsWith("-")
      val body = if negative then text.substring(1) else text
      val (whole, fraction) = body.indexOf('.') match
        case -1 => (body, "")
        case i => (body.substring(0, i), body.substring(i + 1))
      if whole.isEmpty && fraction.isEmpty then Left(s"invalid amount: $text")
      else if fraction.length > 4 then Left(s"too many places: $text")
      else
        val w = if whole.isEmpty then Some(0L) else whole.toLongOption
        val f = if fraction.isEmpty then Some(0L) else (fraction + "0000").take(4).toLongOption
        (w, f) match
          case (Some(a), Some(b)) if whole.forall(_.isDigit) && fraction.forall(_.isDigit) =>
            val value = a * Scale + b
            Right(if negative then -value else value)
          case _ => Left(s"invalid amount: $text")
    def unsafeParse(text: String): Amount = parse(text).fold(e => throw new IllegalArgumentException(e), identity)

    extension (a: Amount)
      def minor: Long = a
      def +(b: Amount): Amount = a + b
      def -(b: Amount): Amount = a - b
      def unary_- : Amount = -a
      def *(n: Long): Amount = a * n
      def /(n: Long): Amount = a / n
      def percent(p: Int): Amount = a * p / 100L
      def isZero: Boolean = a == 0L
      def isPositive: Boolean = a > 0L
      def isNegative: Boolean = a < 0L
      def atLeast(b: Amount): Amount = if a >= b then a else b
      def atMost(b: Amount): Amount = if a <= b then a else b
      def compareTo(b: Amount): Int = java.lang.Long.compare(a, b)
      def roundTo(places: Int): Amount =
        val unit = List(10000L, 1000L, 100L, 10L, 1L)(Math.min(places, 4))
        val half = unit / 2
        val q = if a >= 0 then (a + half) / unit else -((-a + half) / unit)
        q * unit
      /** The text with exactly `places` decimals, half up, as a scaled decimal prints. */
      def format(places: Int): String =
        val rounded = a.roundTo(places)
        val negative = rounded < 0
        val abs = if negative then -rounded else rounded
        val whole = abs / Scale
        val frac = abs % Scale
        val digits = Amount.padFraction(frac)
        val shown = if places == 0 then "" else "." + digits.take(places)
        (if negative then "-" else "") + whole.toString + shown
      def render: String = format(2)

    private def padFraction(frac: Long): String =
      val s = frac.toString
      "0" * (4 - s.length) + s

    given Ordering[Amount] = (a, b) => java.lang.Long.compare(a, b)
    given cats.Order[Amount] = (a, b) => java.lang.Long.compare(a, b)
    given cats.Show[Amount] = a => a.render
    given cats.Monoid[Amount] = new cats.Monoid[Amount]:
      def empty = zero
      def combine(a: Amount, b: Amount) = a + b
