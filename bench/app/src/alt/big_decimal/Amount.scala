package meridian.core.amount

/** The amount over BigDecimal, kept to four places: the surface of the fixed-point one, the
  * arithmetic of scala.math.BigDecimal (the --big-decimal flag of bench/app/gen.py). */
object Amounts:
  opaque type Amount = BigDecimal

  object Amount:
    private val Places = 4
    private val Minor = BigDecimal(10000)
    def zero: Amount = BigDecimal(0)
    def ofUnits(units: Long): Amount = BigDecimal(units)
    def ofMinor(minor: Long): Amount = BigDecimal(minor) / Minor
    def parse(text: String): Either[String, Amount] =
      val negative = text.startsWith("-")
      val body = if negative then text.substring(1) else text
      val (whole, fraction) = body.indexOf('.') match
        case -1 => (body, "")
        case i => (body.substring(0, i), body.substring(i + 1))
      if whole.isEmpty && fraction.isEmpty then Left(s"invalid amount: $text")
      else if fraction.length > Places then Left(s"too many places: $text")
      else if !whole.forall(_.isDigit) || !fraction.forall(_.isDigit) then Left(s"invalid amount: $text")
      else
        val value = BigDecimal(if body.startsWith(".") then "0" + body else body)
        Right(if negative then -value else value)
    def unsafeParse(text: String): Amount = parse(text).fold(e => throw new IllegalArgumentException(e), identity)

    extension (a: Amount)
      def minor: Long = (a * Minor).setScale(0, BigDecimal.RoundingMode.DOWN).toLong
      def +(b: Amount): Amount = a + b
      def -(b: Amount): Amount = a - b
      def unary_- : Amount = -a
      def *(n: Long): Amount = a * BigDecimal(n)
      def /(n: Long): Amount = (a / BigDecimal(n)).setScale(Places, BigDecimal.RoundingMode.DOWN)
      def percent(p: Int): Amount = (a * BigDecimal(p) / BigDecimal(100)).setScale(Places, BigDecimal.RoundingMode.DOWN)
      def isZero: Boolean = a.signum == 0
      def isPositive: Boolean = a.signum > 0
      def isNegative: Boolean = a.signum < 0
      def atLeast(b: Amount): Amount = if a >= b then a else b
      def atMost(b: Amount): Amount = if a <= b then a else b
      def compareTo(b: Amount): Int = a.compare(b)
      def roundTo(places: Int): Amount = a.setScale(Math.min(places, Places), BigDecimal.RoundingMode.HALF_UP)
      /** The text with exactly `places` decimals, half up, as a scaled decimal prints. */
      def format(places: Int): String = a.setScale(places, BigDecimal.RoundingMode.HALF_UP).bigDecimal.toPlainString
      def render: String = format(2)

    given Ordering[Amount] = (a, b) => a.compare(b)
    given cats.Order[Amount] = (a, b) => a.compare(b)
    given cats.Show[Amount] = a => a.render
    given cats.Monoid[Amount] = new cats.Monoid[Amount]:
      def empty = zero
      def combine(a: Amount, b: Amount) = a + b
