// The Java platform layer for JavaScript, `java.text`: the symbols scala-java-time's text
// provider reads for month, day, am/pm and era names. English for every locale; the CLDR
// tables of scala-java-locales are not read. JavaScript only: the JVM has the JDK's.
package java.text:

  @jvmClass("java/text/DateFormatSymbols")
  final class DateFormatSymbols(locale: java.util.Locale):
    // The root locale's full names are the abbreviations, as the JDK's CLDR root has them.
    private def root: Boolean = locale.getLanguage.isEmpty
    def getMonths: Array[String] =
      if root then getShortMonths
      else Array("January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December", "")
    def getShortMonths: Array[String] =
      Array("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "")
    def getWeekdays: Array[String] =
      if root then getShortWeekdays
      else Array("", "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday")
    def getShortWeekdays: Array[String] =
      Array("", "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat")
    def getAmPmStrings: Array[String] = Array("AM", "PM")
    def getEras: Array[String] = Array("BC", "AD")
    def getZoneStrings: Array[Array[String]] = emptyArray[Array[String]]

  @jvmClass("java/text/DateFormatSymbols")
  object DateFormatSymbols:
    def getInstance(locale: java.util.Locale): DateFormatSymbols = new DateFormatSymbols(locale)
    def getInstance(): DateFormatSymbols = new DateFormatSymbols(java.util.Locale.getDefault())

  @jvmClass("java/text/DecimalFormatSymbols")
  final class DecimalFormatSymbols(locale: java.util.Locale):
    def getZeroDigit: Char = '0'
    def getMinusSign: Char = '-'
    def getDecimalSeparator: Char = '.'
    def getGroupingSeparator: Char = ','
    def getPercent: Char = '%'
    def getPerMill: Char = '\u2030'
    def getExponentSeparator: String = "E"

  @jvmClass("java/text/DecimalFormatSymbols")
  object DecimalFormatSymbols:
    def getInstance(locale: java.util.Locale): DecimalFormatSymbols = new DecimalFormatSymbols(locale)
    def getInstance(): DecimalFormatSymbols = new DecimalFormatSymbols(java.util.Locale.getDefault())
    def getAvailableLocales: Array[java.util.Locale] = Array(java.util.Locale.ROOT, java.util.Locale.ENGLISH, java.util.Locale.US)

  @jvmClass("java/text/ParsePosition")
  class ParsePosition(private var index: Int):
    private var errorIndex: Int = -1
    def getIndex: Int = index
    def setIndex(i: Int): Unit = index = i
    def getErrorIndex: Int = errorIndex
    def setErrorIndex(i: Int): Unit = errorIndex = i
    override def equals(that: Any): Boolean = that match
      case p: ParsePosition => p.getIndex == index && p.getErrorIndex == errorIndex
      case _ => false
    override def hashCode: Int = errorIndex * 31 + index
    override def toString: String = "java.text.ParsePosition[index=" + index + ",errorIndex=" + errorIndex + "]"

  @jvmClass("java/text/FieldPosition")
  class FieldPosition(field: Int):
    private var begin = 0
    private var end = 0
    def getField: Int = field
    def getBeginIndex: Int = begin
    def getEndIndex: Int = end
    def setBeginIndex(i: Int): Unit = begin = i
    def setEndIndex(i: Int): Unit = end = i

  @jvmClass("java/text/Format")
  abstract class Format:
    def format(obj: AnyRef): String = format(obj, new java.lang.StringBuffer(), new FieldPosition(0)).toString
    def format(obj: AnyRef, toAppendTo: java.lang.StringBuffer, pos: FieldPosition): java.lang.StringBuffer

  /** As scala-java-locales has it, the `java.text` of a Scala.js build: a number whose Long and
    * Double values agree is formatted as the Long, any other as the Double. */
  @jvmClass("java/text/NumberFormat")
  abstract class NumberFormat extends Format:
    private var roundingMode = java.math.RoundingMode.HALF_EVEN
    override def format(obj: AnyRef, toAppendTo: java.lang.StringBuffer, pos: FieldPosition): java.lang.StringBuffer = obj match
      case n: java.lang.Number if n.doubleValue == n.longValue.toDouble => format(n.longValue, toAppendTo, pos)
      case n: java.lang.Number => format(n.doubleValue, toAppendTo, pos)
      case _ => throw new IllegalArgumentException("Cannot format given Object as a Number")
    final def format(number: Double): String = format(number, new java.lang.StringBuffer(), new FieldPosition(0)).toString
    final def format(number: Long): String = format(number, new java.lang.StringBuffer(), new FieldPosition(0)).toString
    def format(number: Double, toAppendTo: java.lang.StringBuffer, pos: FieldPosition): java.lang.StringBuffer
    def format(number: Long, toAppendTo: java.lang.StringBuffer, pos: FieldPosition): java.lang.StringBuffer
    def getRoundingMode(): java.math.RoundingMode = roundingMode
    def setRoundingMode(mode: java.math.RoundingMode): Unit = roundingMode = mode
    def isGroupingUsed(): Boolean
    def setGroupingUsed(newValue: Boolean): Unit
    def getMaximumIntegerDigits(): Int
    def setMaximumIntegerDigits(newValue: Int): Unit
    def getMinimumIntegerDigits(): Int
    def setMinimumIntegerDigits(newValue: Int): Unit
    def getMaximumFractionDigits(): Int
    def setMaximumFractionDigits(newValue: Int): Unit
    def getMinimumFractionDigits(): Int
    def setMinimumFractionDigits(newValue: Int): Unit

  @jvmClass("java/text/NumberFormat")
  object NumberFormat:
    val INTEGER_FIELD: Int = 0
    val FRACTION_FIELD: Int = 1
    def getInstance(): NumberFormat = getNumberInstance()
    def getInstance(locale: java.util.Locale): NumberFormat = getNumberInstance(locale)
    def getNumberInstance(): NumberFormat = new DecimalFormat("#,##0.###")
    def getNumberInstance(locale: java.util.Locale): NumberFormat = new DecimalFormat("#,##0.###", DecimalFormatSymbols.getInstance(locale))
    def getIntegerInstance(): NumberFormat = getIntegerInstance(java.util.Locale.getDefault())
    def getIntegerInstance(locale: java.util.Locale): NumberFormat =
      val f = new DecimalFormat("#,##0", DecimalFormatSymbols.getInstance(locale))
      f.setMaximumFractionDigits(0)
      f
    def getPercentInstance(): NumberFormat = getPercentInstance(java.util.Locale.getDefault())
    def getPercentInstance(locale: java.util.Locale): NumberFormat =
      val f = new DecimalFormat("#,##0%", DecimalFormatSymbols.getInstance(locale))
      f.setMaximumFractionDigits(0)
      f

  /** A pattern read and a number formatted as scala-java-locales does, the `java.text` of a
    * Scala.js build: a Double from its shortest decimal form (`BigDecimal.valueOf`), so a tie is a
    * decimal one (2.345 to two places gives 2.34 under HALF_EVEN, where the JDK rounds the binary
    * value up); a pattern without a decimal point allows five fraction digits and one without
    * minimum fraction digits a minimum of one integer digit; a zero integer part is written `0`.
    * In scientific notation the mantissa keeps the minimum integer and maximum fraction digits'
    * precision, a pure fraction rounding half up, and the exponent is written without padding. */
  @jvmClass("java/text/DecimalFormat")
  class DecimalFormat(pattern: String, private var symbols: DecimalFormatSymbols) extends NumberFormat:
    def this(pattern: String) = this(pattern, DecimalFormatSymbols.getInstance())
    def this() = this("#,##0.###")

    // -1 where the pattern leaves a count open, as scala-java-locales' `None`.
    private var positivePrefix = ""
    private var positiveSuffix = ""
    private var negativePrefix: String = null
    private var negativeSuffix: String = null
    private var hasNegative = false
    private var multiplier = 1
    private var groupingSize = 0
    private var groupingUsed = false
    private var minInt = -1
    private var maxInt = -1
    private var minFrac = -1
    private var maxFrac = -1
    private var minExp = -1
    private var separatorAlwaysShown = false
    applyPattern(pattern)

    def applyPattern(p: String): Unit =
      val semicolon = p.indexOf(';')
      val positive = DecimalFormat.split(if semicolon < 0 then p else p.substring(0, semicolon))
      val negative = if semicolon < 0 || semicolon == p.length - 1 then null else DecimalFormat.split(p.substring(semicolon + 1))
      val body = positive(1)
      val exponent = body.indexOf('E')
      positivePrefix = DecimalFormat.nonBlank(positive(0))
      positiveSuffix = DecimalFormat.nonBlank(positive(2))
      hasNegative = negative != null
      negativePrefix = if hasNegative then DecimalFormat.nonBlank(negative(0)) else null
      negativeSuffix = if hasNegative then DecimalFormat.nonBlank(negative(2)) else null
      if negativePrefix != null && negativePrefix.isEmpty then negativePrefix = null
      if negativeSuffix != null && negativeSuffix.isEmpty then negativeSuffix = null
      multiplier = if positiveSuffix.indexOf('%') >= 0 then 100 else if positiveSuffix.indexOf('\u2030') >= 0 then 1000 else 1
      val point = body.indexOf('.')
      minFrac = if point < 0 then -1 else DecimalFormat.leadingZeros(body.substring(point + 1))
      maxInt =
        if exponent < 0 then -1
        else if point <= 0 then throw new AssertionError("assertion failed: Exponent pattern must have a decimal")
        else point
      minInt =
        val before = if point < 0 then -1 else DecimalFormat.leadingZeros(body.substring(0, point).reverse)
        if exponent >= 0 && before >= 0 && maxInt > before && maxInt > 1 then 1
        else if before >= 0 then before
        else if minFrac >= 0 then -1
        else 1
      minExp = if exponent < 0 then -1 else DecimalFormat.leadingZeros(body.substring(exponent + 1))
      maxFrac =
        if point < 0 then -1
        else body.substring(point + 1).takeWhile(c => c == '0' || c == '#' || c == ',').count(c => c == '0' || c == '#')
      groupingSize = DecimalFormat.groupingCount(body)
      groupingUsed = groupingSize > 0

    private def localized(affix: String): String =
      affix.replace('%', symbols.getPercent).replace('\u2030', symbols.getPerMill)
    def getPositivePrefix(): String = localized(positivePrefix)
    def setPositivePrefix(newValue: String): Unit = positivePrefix = if newValue == null then "" else newValue
    def getPositiveSuffix(): String = localized(positiveSuffix)
    def setPositiveSuffix(newValue: String): Unit = positiveSuffix = if newValue == null then "" else newValue
    def getNegativePrefix(): String =
      val minus = symbols.getMinusSign.toString
      localized(if negativePrefix != null then negativePrefix else if !hasNegative && positivePrefix.nonEmpty then minus + positivePrefix else minus)
    def setNegativePrefix(newValue: String): Unit = negativePrefix = newValue
    def getNegativeSuffix(): String =
      localized(if negativeSuffix != null then negativeSuffix else if !hasNegative then positiveSuffix else "")
    def setNegativeSuffix(newValue: String): Unit = negativeSuffix = newValue
    def getMultiplier(): Int = multiplier
    def setMultiplier(newValue: Int): Unit = multiplier = newValue
    def getGroupingSize(): Int = groupingSize
    def setGroupingSize(newValue: Int): Unit =
      if newValue < 0 then throw new IllegalArgumentException("")
      groupingSize = newValue
      groupingUsed = newValue > 0
    override def isGroupingUsed(): Boolean = groupingUsed
    override def setGroupingUsed(newValue: Boolean): Unit = groupingUsed = newValue
    def isDecimalSeparatorAlwaysShown(): Boolean = separatorAlwaysShown
    def setDecimalSeparatorAlwaysShown(newValue: Boolean): Unit = separatorAlwaysShown = newValue
    def getDecimalFormatSymbols(): DecimalFormatSymbols = symbols
    def setDecimalFormatSymbols(newSymbols: DecimalFormatSymbols): Unit = symbols = newSymbols
    override def getMaximumIntegerDigits(): Int = if maxInt < 0 then Int.MaxValue else maxInt
    override def setMaximumIntegerDigits(newValue: Int): Unit =
      maxInt = Math.max(newValue, 0)
      if minInt >= 0 then minInt = Math.min(minInt, maxInt)
    override def getMinimumIntegerDigits(): Int = Math.max(minInt, 0)
    override def setMinimumIntegerDigits(newValue: Int): Unit =
      minInt = Math.max(newValue, 0)
      if maxInt >= 0 then maxInt = Math.max(maxInt, minInt)
    override def getMaximumFractionDigits(): Int = if maxFrac < 0 then 5 else maxFrac
    override def setMaximumFractionDigits(newValue: Int): Unit =
      maxFrac = Math.max(newValue, 0)
      if minFrac >= 0 then minFrac = Math.min(minFrac, maxFrac)
    override def getMinimumFractionDigits(): Int = Math.max(minFrac, 0)
    override def setMinimumFractionDigits(newValue: Int): Unit =
      minFrac = Math.max(newValue, 0)
      if maxFrac >= 0 then maxFrac = Math.max(maxFrac, minFrac)

    override def format(number: Double, toAppendTo: java.lang.StringBuffer, pos: FieldPosition): java.lang.StringBuffer =
      format(java.math.BigDecimal.valueOf(number), toAppendTo)
    override def format(number: Long, toAppendTo: java.lang.StringBuffer, pos: FieldPosition): java.lang.StringBuffer =
      format(java.math.BigDecimal.valueOf(number), toAppendTo)

    private def format(number: java.math.BigDecimal, out: java.lang.StringBuffer): java.lang.StringBuffer =
      if minExp >= 0 then formatScientific(number, out) else formatDecimal(number, out)

    private def isExponentPowerMultiple: Boolean =
      getMaximumIntegerDigits() > getMinimumIntegerDigits() && getMaximumIntegerDigits() > 1

    private def exponentPrecision: Int =
      if isExponentPowerMultiple then getMaximumIntegerDigits() + getMaximumFractionDigits()
      else getMinimumIntegerDigits() + getMaximumFractionDigits()

    /** The mantissa with the precision the pattern allows, and the power of ten it is scaled by. */
    private def mantissaAndPower(n: java.math.BigDecimal): (java.math.BigDecimal, Int) =
      if n.signum == 0 then (java.math.BigDecimal.ZERO, 0)
      else
        val justFraction = n.abs.compareTo(java.math.BigDecimal.ONE) < 0
        val pointAt = n.precision - n.scale
        val precision = Math.min(n.precision, exponentPrecision)
        val maxI = getMaximumIntegerDigits()
        val integerSize =
          if isExponentPowerMultiple then
            var idx = 0
            while idx < maxI && (pointAt - (maxI - idx)) % maxI != 0 do idx += 1
            if idx < maxI then maxI - idx else 1
          else Math.max(Math.min(precision, maxI), 1)
        val unscaled = new java.math.BigDecimal(n.unscaledValue, precision - integerSize)
        val mode = if justFraction then java.math.RoundingMode.HALF_UP else getRoundingMode()
        (unscaled.divide(java.math.BigDecimal.TEN.pow(n.precision - precision), mode), pointAt - integerSize)

    /** The digits of `n` from the last, the integer part's stopping at the maximum integer digits. */
    private def reversedDigits(n: java.math.BigInteger, out: java.lang.StringBuilder, integerPart: Boolean): Int =
      var written = 0
      var rest = n
      def total = if groupingUsed && groupingSize > 0 && written > 0 then written + written / groupingSize else written
      while rest.signum > 0 && (!integerPart || total <= getMaximumIntegerDigits()) do
        out.append((rest.remainder(java.math.BigInteger.TEN).intValue + symbols.getZeroDigit).toChar)
        rest = rest.divide(java.math.BigInteger.TEN)
        written += 1
      written

    private def formatScientific(number: java.math.BigDecimal, out: java.lang.StringBuffer): java.lang.StringBuffer =
      val negative = number.signum < 0
      out.append(if negative then getNegativePrefix() else getPositivePrefix())
      val (mantissa, power) = mantissaAndPower(number.multiply(java.math.BigDecimal.valueOf(multiplier.toLong)).abs)
      val zero = symbols.getZeroDigit
      val integerPart = new java.math.BigDecimal(mantissa.toBigInteger, 0)
      val integer = new java.lang.StringBuilder()
      var written =
        if integerPart.signum == 0 || getMaximumIntegerDigits() == 0 then
          integer.append(zero)
          1
        else reversedDigits(integerPart.toBigInteger, integer, true)
      while written < getMinimumIntegerDigits() do
        integer.append(zero)
        written += 1
      out.append(integer.reverse().toString)
      if mantissa.compareTo(integerPart) > 0 then
        out.append(symbols.getDecimalSeparator)
        val maxDigits = exponentPrecision - written
        val digits = new java.lang.StringBuilder()
        reversedDigits(mantissa.setScale(maxDigits, getRoundingMode()).subtract(integerPart).unscaledValue, digits, false)
        val reversed = digits.toString
        val fraction = (reversed.dropWhile(_ == zero) + zero.toString * (maxDigits - reversed.length)).reverse
        out.append(fraction)
        if fraction.length < getMinimumFractionDigits() then out.append(zero.toString * (getMinimumFractionDigits() - fraction.length))
      else if getMinimumFractionDigits() > 0 then
        out.append(symbols.getDecimalSeparator)
        out.append(zero.toString * getMinimumFractionDigits())
      out.append(symbols.getExponentSeparator)
      out.append(power.toString)
      out.append(if negative then getNegativeSuffix() else getPositiveSuffix())

    private def separatorBefore(out: java.lang.StringBuilder, written: Int): Unit =
      if groupingUsed && groupingSize > 0 && written > 0 && written % groupingSize == 0 then out.append(symbols.getGroupingSeparator)

    private def formatDecimal(number: java.math.BigDecimal, out: java.lang.StringBuffer): java.lang.StringBuffer =
      val negative = number.signum < 0
      out.append(if negative then getNegativePrefix() else getPositivePrefix())
      val maxF = getMaximumFractionDigits()
      val minF = getMinimumFractionDigits()
      val scaled = number.multiply(java.math.BigDecimal.valueOf(multiplier.toLong)).abs.setScale(Math.max(maxF, minF), getRoundingMode())
      val plain = scaled.toPlainString
      val point = plain.indexOf('.')
      val integerDigits = (if point < 0 then plain else plain.substring(0, point)).dropWhile(_ == '0')
      val fractionDigits = if point < 0 then "" else plain.substring(point + 1)
      val zero = symbols.getZeroDigit
      // The integer part is written from its last digit, as scala-java-locales writes it.
      val reversed = new java.lang.StringBuilder()
      var written = 0
      if integerDigits.isEmpty || getMaximumIntegerDigits() == 0 then
        reversed.append(zero)
        written = 1
      else
        var i = integerDigits.length - 1
        def total = if groupingUsed && groupingSize > 0 && written > 0 then written + written / groupingSize else written
        while i >= 0 && total <= getMaximumIntegerDigits() do
          separatorBefore(reversed, written)
          reversed.append((integerDigits.charAt(i) - '0' + zero).toChar)
          written += 1
          i -= 1
      while written < getMinimumIntegerDigits() do
        separatorBefore(reversed, written)
        reversed.append(zero)
        written += 1
      out.append(reversed.reverse().toString)
      val fraction = fractionDigits.take(maxF).reverse.dropWhile(_ == '0').reverse
      if fraction.nonEmpty then
        out.append(symbols.getDecimalSeparator)
        out.append(fraction.map(c => (c - '0' + zero).toChar))
        if fraction.length < minF then out.append(zero.toString * (minF - fraction.length))
      else if minF > 0 then
        out.append(symbols.getDecimalSeparator)
        out.append(zero.toString * minF)
      out.append(if negative then getNegativeSuffix() else getPositiveSuffix())

  @jvmClass("java/text/DecimalFormat")
  object DecimalFormat:
    private val numberChars = "0#.-,E"

    /** The prefix, the number part and the suffix of a subpattern, as scala-java-locales splits
      * it: a quoted run goes to the affix it stands in, `''` is a quote. */
    private[text] def split(p: String): Array[String] =
      val prefix = new java.lang.StringBuilder()
      val body = new java.lang.StringBuilder()
      val suffix = new java.lang.StringBuilder()
      var prefixReady = false
      var bodyReady = false
      var i = 0
      while i < p.length do
        val c = p.charAt(i)
        if c == '\'' then
          val close = p.indexOf('\'', i + 1)
          if close < 0 then throw new RuntimeException()
          val quoted = if close == i + 1 then "'" else p.substring(i + 1, close)
          if !prefixReady then prefix.append(quoted)
          else
            bodyReady = true
            suffix.append(quoted)
          i = close + 1
        else
          val inNumber = numberChars.indexOf(c) >= 0
          if !prefixReady && !inNumber then prefix.append(c)
          else if !prefixReady then prefixReady = true
          if prefixReady && inNumber then body.append(c)
          else if prefixReady then bodyReady = true
          if bodyReady && !inNumber then suffix.append(c)
          i += 1
      Array(prefix.toString, body.toString, suffix.toString)

    private[text] def nonBlank(s: String): String = if s.forall(Character.isWhitespace) then "" else s

    /** The zeros a digit run starts with, grouping separators skipped; -1 for none. */
    private[text] def leadingZeros(digits: String): Int =
      val n = digits.filter(_ != ',').takeWhile(_ == '0').length
      if n > 0 then n else -1

    private[text] def groupingCount(body: String): Int =
      val pat = body.filter(c => "0,.\u2030#;E-".indexOf(c) >= 0)
      val decimal = pat.lastIndexOf('.')
      val beforeDecimal = if decimal < 0 then pat.length - 1 else decimal - 1
      val separator = pat.substring(0, Math.max(beforeDecimal + 1, 0)).lastIndexOf(',')
      if pat.isEmpty then 3
      else if separator < 0 then 0
      else if decimal < 0 then pat.length - 1 - separator
      else beforeDecimal - separator
