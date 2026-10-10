// The Java platform layer for JavaScript: the members of `java.lang` that library bodies read
// from TASTy call. With the std present a reference to a Java class binds to the class here by
// qualified name before the JDK's signatures are consulted, so on JavaScript these are the
// implementations; on the JVM the classes are the JDK's own (`@jvmClass`) and every member is
// the JDK's method. Under `--std=scala-library` the layer stays and scala-library's bodies call it.
package java.lang:

  type Object = AnyRef

  @jvmClass("java/lang/Cloneable")
  trait Cloneable

  @jvmClass("java/lang/AutoCloseable")
  trait AutoCloseable:
    def close(): Unit

  @jvmClass("java/lang/Readable")
  trait Readable

  @jvmClass("java/lang/Runnable")
  trait Runnable:
    def run(): Unit

  @jvmClass("java/lang/Comparable")
  trait Comparable[T]:
    // The boxes of the primitives and strings are JS values, compared as Scala.js's dispatch
    // does: every number as a Double.
    @javaDefined
    @js("$compareTo($0, $1)")
    @jvm("$0:L checkcast java/lang/Comparable $1:L invokeinterface java/lang/Comparable.compareTo(Ljava/lang/Object;)I")
    def compareTo(that: T): Int

  @jvmClass("java/lang/Iterable")
  trait Iterable[T]:
    def iterator(): java.util.Iterator[T]
    def forEach(action: java.util.function.Consumer[T]): Unit =
      val it = iterator()
      while it.hasNext do action.accept(it.next())

  // `append` takes the JDK's three overloads' arguments as one, since a `StringBuilder`
  // appends anything.
  @jvmClass("java/lang/Appendable")
  trait Appendable:
    def append(x: Any): Appendable

  // An enum over it, a program's or a jar's, is one of teq's enums: its values carry their
  // name and ordinal (`$name`, `$ordinal`), which these read; the JVM has the JDK's class.
  @javaDefined
  @jvmClass("java/lang/Enum")
  abstract class Enum[E <: Enum[E]] extends Comparable[E]:
    // A class written Java-style passes its name and ordinal (`extends Enum[X](name, ordinal)`).
    def this(name: String, ordinal: Int) =
      this()
      named(name, ordinal)
    @js("void ($0.$name = $1, $0.$ordinal = $2)")
    private def named(name: String, ordinal: Int): Unit
    @js("$0.$name")
    @jvm("invokevirtual java/lang/Enum.name()Ljava/lang/String;")
    def name(): String
    @js("$0.$ordinal")
    @jvm("invokevirtual java/lang/Enum.ordinal()I")
    def ordinal(): Int
    // Called by name through `Comparable`, `$str` and the collections: methods, not templates.
    @jvm("invokevirtual java/lang/Enum.compareTo(Ljava/lang/Enum;)I")
    def compareTo(that: E): Int = ordinal() - that.ordinal()
    @jvm("invokevirtual java/lang/Enum.toString()Ljava/lang/String;")
    override def toString: String = name()
    @jvm("invokevirtual java/lang/Enum.hashCode()I")
    override def hashCode: Int = name().hashCode
    @jvm("invokevirtual java/lang/Enum.equals(Ljava/lang/Object;)Z")
    override def equals(that: Any): scala.Boolean = this eq that.asInstanceOf[AnyRef]

  @javaDefined
  @jvmClass("java/lang/Enum")
  object Enum:
    // A class carries no table of its enum's values on JavaScript.
    @jvm("invokestatic java/lang/Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;")
    def valueOf[T <: Enum[T]](enumClass: Class[T], name: String): T =
      throw new UnsupportedOperationException("Enum.valueOf is not supported on JavaScript")

  @jvmClass("java/lang/CharSequence")
  trait CharSequence:
    // A `String` is a JS string, whose length is a property.
    @javaDefined
    @js("(typeof $0 === \"string\" ? $0.length : $0.length())")
    @jvm("invokeinterface java/lang/CharSequence.length()I")
    def length: Int
    @javaDefined
    @js("(typeof $0 === \"string\" ? $charAt($0, $1) : $0.charAt($1))")
    @jvm("invokeinterface java/lang/CharSequence.charAt(I)C")
    def charAt(index: Int): Char
    @javaDefined
    @js("(typeof $0 === \"string\" ? $0.length === 0 : $0.length() === 0)")
    @jvm("invokeinterface java/lang/CharSequence.isEmpty()Z")
    def isEmpty: scala.Boolean = length == 0
    // The JDK's `StringBuilder` answers a `String` too.
    @javaDefined
    @js("(typeof $0 === \"string\" ? $0 : $0.toString()).substring($1, $2)")
    @jvm("invokeinterface java/lang/CharSequence.subSequence(II)Ljava/lang/CharSequence;")
    def subSequence(start: Int, end: Int): CharSequence

  @javaDefined
  @jvmClass("java/lang/Number")
  abstract class Number:
    // The boxes of the numeric primitives are JS numbers and, for `Long`, bigints.
    @javaDefined
    @js("(typeof $0 === \"number\" ? $d2i($0) : typeof $0 === \"bigint\" ? Number(BigInt.asIntN(32, $0)) : $0.intValue())")
    @jvm("invokevirtual java/lang/Number.intValue()I")
    def intValue: Int
    @javaDefined
    @js("(typeof $0 === \"number\" ? $d2l($0) : typeof $0 === \"bigint\" ? $0 : $0.longValue())")
    @jvm("invokevirtual java/lang/Number.longValue()J")
    def longValue: scala.Long
    @javaDefined
    @js("(typeof $0 === \"number\" || typeof $0 === \"bigint\" ? Math.fround(Number($0)) : $0.floatValue())")
    @jvm("invokevirtual java/lang/Number.floatValue()F")
    def floatValue: scala.Float
    @javaDefined
    @js("(typeof $0 === \"number\" || typeof $0 === \"bigint\" ? Number($0) : $0.doubleValue())")
    @jvm("invokevirtual java/lang/Number.doubleValue()D")
    def doubleValue: scala.Double
    @javaDefined
    @js("((typeof $0 === \"number\" ? $d2i($0) : typeof $0 === \"bigint\" ? Number(BigInt.asIntN(32, $0)) : $0.intValue()) << 24 >> 24)")
    @jvm("invokevirtual java/lang/Number.byteValue()B")
    def byteValue: scala.Byte = intValue.toByte
    @javaDefined
    @js("((typeof $0 === \"number\" ? $d2i($0) : typeof $0 === \"bigint\" ? Number(BigInt.asIntN(32, $0)) : $0.intValue()) << 16 >> 16)")
    @jvm("invokevirtual java/lang/Number.shortValue()S")
    def shortValue: scala.Short = intValue.toShort

  @javaDefined
  @jvmClass("java/lang/Integer")
  final class Integer private (value: Int) extends Number, Comparable[Integer]:
    @js("Math.fround($0)")
    @jvm("invokevirtual java/lang/Integer.floatValue()F")
    def floatValue: scala.Float
    @js("$0")
    @jvm("invokevirtual java/lang/Integer.intValue()I")
    def intValue: Int
    @js("$0")
    @jvm("invokevirtual java/lang/Integer.longValue()J")
    def longValue: Long
    @js("$0")
    @jvm("invokevirtual java/lang/Integer.doubleValue()D")
    def doubleValue: Double
    @js("($0 < $1 ? -1 : $0 > $1 ? 1 : 0)")
    @jvm("invokevirtual java/lang/Integer.compareTo(Ljava/lang/Integer;)I")
    def compareTo(that: Integer): Int

  @javaDefined
  @jvmClass("java/lang/Integer")
  object Integer:
    // `new Integer(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Integer dup $1:I invokespecial java/lang/Integer.<init>(I)V")
    def newInteger(value: Int): Integer = Integer.valueOf(value)
    @jvm("new java/lang/Integer dup $1 invokespecial java/lang/Integer.<init>(Ljava/lang/String;)V")
    def newInteger(s: String): Integer = Integer.valueOf(parseInt(s))
    @jvm("getstatic java/lang/Integer.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[Int]
    // The JDK's constants (`ConstantValue`), of their constant types as dotty reads them.
    final val MAX_VALUE: 2147483647 = 2147483647
    final val MIN_VALUE: -2147483648 = -2147483648
    final val SIZE: 32 = 32
    @js("$1")
    @jvm("invokestatic java/lang/Integer.valueOf(I)Ljava/lang/Integer;")
    def valueOf(i: Int): Integer
    @js("$parseIntRadix($1, 10)")
    @jvm("invokestatic java/lang/Integer.valueOf(Ljava/lang/String;)Ljava/lang/Integer;")
    def valueOf(s: String): Integer
    @js("$parseIntRadix($1, 10)")
    @jvm("invokestatic java/lang/Integer.parseInt(Ljava/lang/String;)I")
    def parseInt(s: String): Int
    @js("$parseIntRadix($1, $2)")
    @jvm("invokestatic java/lang/Integer.parseInt(Ljava/lang/String;I)I")
    def parseInt(s: String, radix: Int): Int
    @js("(\"\" + $1)")
    @jvm("invokestatic java/lang/Integer.toString(I)Ljava/lang/String;")
    def toString(i: Int): String
    @js("($1).toString($2)")
    @jvm("invokestatic java/lang/Integer.toString(II)Ljava/lang/String;")
    def toString(i: Int, radix: Int): String
    @js("(($1) >>> 0).toString(16)")
    @jvm("invokestatic java/lang/Integer.toHexString(I)Ljava/lang/String;")
    def toHexString(i: Int): String
    @js("(($1) >>> 0).toString(2)")
    @jvm("invokestatic java/lang/Integer.toBinaryString(I)Ljava/lang/String;")
    def toBinaryString(i: Int): String
    @js("(($1) >>> 0).toString(8)")
    @jvm("invokestatic java/lang/Integer.toOctalString(I)Ljava/lang/String;")
    def toOctalString(i: Int): String
    @js("$1")
    @jvm("invokestatic java/lang/Integer.hashCode(I)I")
    def hashCode(i: Int): Int
    @js("($1 < $2 ? -1 : $1 > $2 ? 1 : 0)")
    @jvm("invokestatic java/lang/Integer.compare(II)I")
    def compare(a: Int, b: Int): Int
    @js("(($1 >>> 0) < ($2 >>> 0) ? -1 : ($1 >>> 0) > ($2 >>> 0) ? 1 : 0)")
    @jvm("invokestatic java/lang/Integer.compareUnsigned(II)I")
    def compareUnsigned(a: Int, b: Int): Int
    @js("((($1 >>> 0) / ($2 >>> 0)) | 0)")
    @jvm("invokestatic java/lang/Integer.divideUnsigned(II)I")
    def divideUnsigned(a: Int, b: Int): Int
    @js("((($1 >>> 0) % ($2 >>> 0)) | 0)")
    @jvm("invokestatic java/lang/Integer.remainderUnsigned(II)I")
    def remainderUnsigned(a: Int, b: Int): Int
    @js("$bitCount($1)")
    @jvm("invokestatic java/lang/Integer.bitCount(I)I")
    def bitCount(i: Int): Int
    @js("Math.clz32($1)")
    @jvm("invokestatic java/lang/Integer.numberOfLeadingZeros(I)I")
    def numberOfLeadingZeros(i: Int): Int
    @js("($1 === 0 ? 32 : 31 - Math.clz32($1 & -$1))")
    @jvm("invokestatic java/lang/Integer.numberOfTrailingZeros(I)I")
    def numberOfTrailingZeros(i: Int): Int
    @js("($1 === 0 ? 0 : (1 << (31 - Math.clz32($1))))")
    @jvm("invokestatic java/lang/Integer.highestOneBit(I)I")
    def highestOneBit(i: Int): Int
    @js("($1 & -$1)")
    @jvm("invokestatic java/lang/Integer.lowestOneBit(I)I")
    def lowestOneBit(i: Int): Int
    @js("(($1 << $2) | ($1 >>> -$2))")
    @jvm("invokestatic java/lang/Integer.rotateLeft(II)I")
    def rotateLeft(i: Int, distance: Int): Int
    @js("(($1 >>> $2) | ($1 << -$2))")
    @jvm("invokestatic java/lang/Integer.rotateRight(II)I")
    def rotateRight(i: Int, distance: Int): Int
    @js("$reverseBits($1)")
    @jvm("invokestatic java/lang/Integer.reverse(I)I")
    def reverse(i: Int): Int
    @js("((($1 & 0xff) << 24) | (($1 & 0xff00) << 8) | (($1 >>> 8) & 0xff00) | ($1 >>> 24))")
    @jvm("invokestatic java/lang/Integer.reverseBytes(I)I")
    def reverseBytes(i: Int): Int
    @js("($1 > 0 ? 1 : $1 < 0 ? -1 : 0)")
    @jvm("invokestatic java/lang/Integer.signum(I)I")
    def signum(i: Int): Int
    @js("Math.max($1, $2)")
    @jvm("invokestatic java/lang/Integer.max(II)I")
    def max(a: Int, b: Int): Int
    @js("Math.min($1, $2)")
    @jvm("invokestatic java/lang/Integer.min(II)I")
    def min(a: Int, b: Int): Int
    @js("(($1 + $2) | 0)")
    @jvm("invokestatic java/lang/Integer.sum(II)I")
    def sum(a: Int, b: Int): Int
    @js("$toLong($1 >>> 0)")
    @jvm("invokestatic java/lang/Integer.toUnsignedLong(I)J")
    def toUnsignedLong(i: Int): Long
    @js("(\"\" + ($1 >>> 0))")
    @jvm("invokestatic java/lang/Integer.toUnsignedString(I)Ljava/lang/String;")
    def toUnsignedString(i: Int): String

  @javaDefined
  @jvmClass("java/lang/Long")
  final class Long private (value: scala.Long) extends Number, Comparable[Long]:
    @js("Math.fround(Number($0))")
    @jvm("invokevirtual java/lang/Long.floatValue()F")
    def floatValue: scala.Float
    @js("Number($0)")
    @jvm("invokevirtual java/lang/Long.doubleValue()D")
    def doubleValue: scala.Double
    @js("$longToInt($0)")
    @jvm("invokevirtual java/lang/Long.intValue()I")
    def intValue: Int
    @js("$0")
    @jvm("invokevirtual java/lang/Long.longValue()J")
    def longValue: scala.Long
    @js("($0 < $1 ? -1 : $0 > $1 ? 1 : 0)")
    @jvm("invokevirtual java/lang/Long.compareTo(Ljava/lang/Long;)I")
    def compareTo(that: Long): Int

  @javaDefined
  @jvmClass("java/lang/Long")
  object Long:
    // `new Long(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Long dup $1:J invokespecial java/lang/Long.<init>(J)V")
    def newLong(value: scala.Long): Long = Long.valueOf(value)
    @jvm("new java/lang/Long dup $1 invokespecial java/lang/Long.<init>(Ljava/lang/String;)V")
    def newLong(s: String): Long = Long.valueOf(parseLong(s))
    @jvm("getstatic java/lang/Long.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[scala.Long]
    final val MAX_VALUE: 9223372036854775807L = 9223372036854775807L
    final val MIN_VALUE: -9223372036854775808L = -9223372036854775808L
    final val SIZE: 64 = 64
    @js("$1")
    @jvm("invokestatic java/lang/Long.valueOf(J)Ljava/lang/Long;")
    def valueOf(l: scala.Long): Long
    @js("$parseLongUnchecked($1)")
    @jvm("invokestatic java/lang/Long.parseLong(Ljava/lang/String;)J")
    def parseLong(s: String): scala.Long
    @js("(\"\" + $1)")
    @jvm("invokestatic java/lang/Long.toString(J)Ljava/lang/String;")
    def toString(l: scala.Long): String
    @js("($1).toString($2)")
    @jvm("invokestatic java/lang/Long.toString(JI)Ljava/lang/String;")
    def toString(l: scala.Long, radix: Int): String
    @js("(BigInt.asUintN(64, $1)).toString(16)")
    @jvm("invokestatic java/lang/Long.toHexString(J)Ljava/lang/String;")
    def toHexString(l: scala.Long): String
    @js("(BigInt.asUintN(64, $1)).toString(2)")
    @jvm("invokestatic java/lang/Long.toBinaryString(J)Ljava/lang/String;")
    def toBinaryString(l: scala.Long): String
    @js("$longHash($1)")
    @jvm("invokestatic java/lang/Long.hashCode(J)I")
    def hashCode(l: scala.Long): Int
    @js("($1 < $2 ? -1 : $1 > $2 ? 1 : 0)")
    @jvm("invokestatic java/lang/Long.compare(JJ)I")
    def compare(a: scala.Long, b: scala.Long): Int
    @jvm("invokestatic java/lang/Long.compareUnsigned(JJ)I")
    def compareUnsigned(a: scala.Long, b: scala.Long): Int = compare(a ^ MIN_VALUE, b ^ MIN_VALUE)
    // The JDK's, after Hacker's Delight 9.3.
    @jvm("invokestatic java/lang/Long.divideUnsigned(JJ)J")
    def divideUnsigned(dividend: scala.Long, divisor: scala.Long): scala.Long =
      if divisor >= 0 then
        val q = (dividend >>> 1) / divisor << 1
        val r = dividend - q * divisor
        q + ((r | ~(r - divisor)) >>> 63)
      else (dividend & ~(dividend - divisor)) >>> 63
    @js("$longBitCount($1)")
    @jvm("invokestatic java/lang/Long.bitCount(J)I")
    def bitCount(l: scala.Long): Int
    @js("$longClz($1)")
    @jvm("invokestatic java/lang/Long.numberOfLeadingZeros(J)I")
    def numberOfLeadingZeros(l: scala.Long): Int
    @js("$longCtz($1)")
    @jvm("invokestatic java/lang/Long.numberOfTrailingZeros(J)I")
    def numberOfTrailingZeros(l: scala.Long): Int
    @js("BigInt.asIntN(64, ($1 << BigInt($2 & 63)) | (BigInt.asUintN(64, $1) >> BigInt(-$2 & 63)))")
    @jvm("invokestatic java/lang/Long.rotateLeft(JI)J")
    def rotateLeft(l: scala.Long, distance: Int): scala.Long
    @js("BigInt.asIntN(64, (BigInt.asUintN(64, $1) >> BigInt($2 & 63)) | ($1 << BigInt(-$2 & 63)))")
    @jvm("invokestatic java/lang/Long.rotateRight(JI)J")
    def rotateRight(l: scala.Long, distance: Int): scala.Long
    // The reverse of each 32-bit half, swapped.
    @js("BigInt.asIntN(64, (BigInt($reverseBits(Number(BigInt.asUintN(64, $1) & 0xffffffffn)) >>> 0) << 32n) | BigInt($reverseBits(Number(BigInt.asUintN(64, $1) >> 32n)) >>> 0))")
    @jvm("invokestatic java/lang/Long.reverse(J)J")
    def reverse(l: scala.Long): scala.Long
    @js("($1 > 0n ? 1 : $1 < 0n ? -1 : 0)")
    @jvm("invokestatic java/lang/Long.signum(J)I")
    def signum(l: scala.Long): Int
    @js("($1 > $2 ? $1 : $2)")
    @jvm("invokestatic java/lang/Long.max(JJ)J")
    def max(a: scala.Long, b: scala.Long): scala.Long
    @js("($1 < $2 ? $1 : $2)")
    @jvm("invokestatic java/lang/Long.min(JJ)J")
    def min(a: scala.Long, b: scala.Long): scala.Long

  @javaDefined
  @jvmClass("java/lang/Short")
  final class Short private (value: scala.Short) extends Number, Comparable[Short]:
    @js("$0")
    @jvm("invokevirtual java/lang/Short.shortValue()S")
    def shortValue: scala.Short
    @js("$0")
    @jvm("invokevirtual java/lang/Short.intValue()I")
    def intValue: Int
    @js("$0")
    @jvm("invokevirtual java/lang/Short.longValue()J")
    def longValue: scala.Long
    @js("Math.fround($0)")
    @jvm("invokevirtual java/lang/Short.floatValue()F")
    def floatValue: scala.Float
    @js("$0")
    @jvm("invokevirtual java/lang/Short.doubleValue()D")
    def doubleValue: scala.Double
    @js("($0 - $1)")
    @jvm("invokevirtual java/lang/Short.compareTo(Ljava/lang/Short;)I")
    def compareTo(that: Short): Int

  @javaDefined
  @jvmClass("java/lang/Short")
  object Short:
    // `new Short(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Short dup $1:S invokespecial java/lang/Short.<init>(S)V")
    def newShort(value: scala.Short): Short = Short.valueOf(value)
    @jvm("new java/lang/Short dup $1 invokespecial java/lang/Short.<init>(Ljava/lang/String;)V")
    def newShort(s: String): Short = Short.valueOf(parseShort(s))
    @jvm("getstatic java/lang/Short.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[scala.Short]
    @js("32767")
    @jvm("getstatic java/lang/Short.MAX_VALUE:S")
    def MAX_VALUE: scala.Short
    @js("-32768")
    @jvm("getstatic java/lang/Short.MIN_VALUE:S")
    def MIN_VALUE: scala.Short
    @js("$1")
    @jvm("invokestatic java/lang/Short.valueOf(S)Ljava/lang/Short;")
    def valueOf(s: scala.Short): Short
    @jvm("invokestatic java/lang/Short.parseShort(Ljava/lang/String;)S")
    def parseShort(s: String): scala.Short =
      val i = Integer.parseInt(s)
      if i < -32768 || i > 32767 then throw new NumberFormatException("Value out of range. Value:\"" + s + "\" Radix:10")
      i.toShort
    @js("(\"\" + $1)")
    @jvm("invokestatic java/lang/Short.toString(S)Ljava/lang/String;")
    def toString(s: scala.Short): String
    @js("$1")
    @jvm("invokestatic java/lang/Short.hashCode(S)I")
    def hashCode(s: scala.Short): Int
    @js("($1 - $2)")
    @jvm("invokestatic java/lang/Short.compare(SS)I")
    def compare(a: scala.Short, b: scala.Short): Int

  @javaDefined
  @jvmClass("java/lang/Byte")
  final class Byte private (value: scala.Byte) extends Number, Comparable[Byte]:
    @js("$0")
    @jvm("invokevirtual java/lang/Byte.byteValue()B")
    def byteValue: scala.Byte
    @js("$0")
    @jvm("invokevirtual java/lang/Byte.intValue()I")
    def intValue: Int
    @js("$0")
    @jvm("invokevirtual java/lang/Byte.longValue()J")
    def longValue: scala.Long
    @js("Math.fround($0)")
    @jvm("invokevirtual java/lang/Byte.floatValue()F")
    def floatValue: scala.Float
    @js("$0")
    @jvm("invokevirtual java/lang/Byte.doubleValue()D")
    def doubleValue: scala.Double
    @js("($0 - $1)")
    @jvm("invokevirtual java/lang/Byte.compareTo(Ljava/lang/Byte;)I")
    def compareTo(that: Byte): Int

  @javaDefined
  @jvmClass("java/lang/Byte")
  object Byte:
    // `new Byte(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Byte dup $1:B invokespecial java/lang/Byte.<init>(B)V")
    def newByte(value: scala.Byte): Byte = Byte.valueOf(value)
    @jvm("new java/lang/Byte dup $1 invokespecial java/lang/Byte.<init>(Ljava/lang/String;)V")
    def newByte(s: String): Byte = Byte.valueOf(parseByte(s))
    @jvm("getstatic java/lang/Byte.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[scala.Byte]
    @js("127")
    @jvm("getstatic java/lang/Byte.MAX_VALUE:B")
    def MAX_VALUE: scala.Byte
    @js("-128")
    @jvm("getstatic java/lang/Byte.MIN_VALUE:B")
    def MIN_VALUE: scala.Byte
    @js("$1")
    @jvm("invokestatic java/lang/Byte.valueOf(B)Ljava/lang/Byte;")
    def valueOf(b: scala.Byte): Byte
    @jvm("invokestatic java/lang/Byte.parseByte(Ljava/lang/String;)B")
    def parseByte(s: String): scala.Byte =
      val i = Integer.parseInt(s)
      if i < -128 || i > 127 then throw new NumberFormatException("Value out of range. Value:\"" + s + "\" Radix:10")
      i.toByte
    @js("(\"\" + $1)")
    @jvm("invokestatic java/lang/Byte.toString(B)Ljava/lang/String;")
    def toString(b: scala.Byte): String
    @js("$1")
    @jvm("invokestatic java/lang/Byte.hashCode(B)I")
    def hashCode(b: scala.Byte): Int
    @js("($1 - $2)")
    @jvm("invokestatic java/lang/Byte.compare(BB)I")
    def compare(a: scala.Byte, b: scala.Byte): Int

  @javaDefined
  @jvmClass("java/lang/Void")
  object Void:
    @jvm("getstatic java/lang/Void.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[Unit]

  @javaDefined
  @jvmClass("java/lang/Double")
  final class Double private (value: scala.Double) extends Number, Comparable[Double]:
    @js("$d2i($0)")
    @jvm("invokevirtual java/lang/Double.intValue()I")
    def intValue: Int
    @js("$d2l($0)")
    @jvm("invokevirtual java/lang/Double.longValue()J")
    def longValue: scala.Long
    @js("Math.fround($0)")
    @jvm("invokevirtual java/lang/Double.floatValue()F")
    def floatValue: scala.Float
    @js("$0")
    @jvm("invokevirtual java/lang/Double.doubleValue()D")
    def doubleValue: scala.Double
    @js("$compareDoubles($0, $1)")
    @jvm("invokevirtual java/lang/Double.compareTo(Ljava/lang/Double;)I")
    def compareTo(that: Double): Int
    @js("Number.isNaN($0)")
    @jvm("invokevirtual java/lang/Double.isNaN()Z")
    def isNaN(): Boolean
    @js("($0 === Infinity || $0 === -Infinity)")
    @jvm("invokevirtual java/lang/Double.isInfinite()Z")
    def isInfinite(): Boolean

  @javaDefined
  @jvmClass("java/lang/Double")
  object Double:
    // `new Double(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Double dup $1:D invokespecial java/lang/Double.<init>(D)V")
    def newDouble(value: scala.Double): Double = Double.valueOf(value)
    @jvm("new java/lang/Double dup $1 invokespecial java/lang/Double.<init>(Ljava/lang/String;)V")
    def newDouble(s: String): Double = Double.valueOf(parseDouble(s))
    @jvm("getstatic java/lang/Double.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[scala.Double]
    final val MAX_VALUE: 1.7976931348623157e308 = 1.7976931348623157e308
    final val MIN_VALUE: 4.9e-324 = 4.9e-324
    @js("Infinity")
    @jvm("getstatic java/lang/Double.POSITIVE_INFINITY:D")
    def POSITIVE_INFINITY: scala.Double
    @js("-Infinity")
    @jvm("getstatic java/lang/Double.NEGATIVE_INFINITY:D")
    def NEGATIVE_INFINITY: scala.Double
    @js("NaN")
    @jvm("getstatic java/lang/Double.NaN:D")
    def NaN: scala.Double
    @js("$1")
    @jvm("invokestatic java/lang/Double.valueOf(D)Ljava/lang/Double;")
    def valueOf(d: scala.Double): Double
    @js("$parseDoubleUnchecked($1)")
    @jvm("invokestatic java/lang/Double.parseDouble(Ljava/lang/String;)D")
    def parseDouble(s: String): scala.Double
    @js("$doubleToString($1)")
    @jvm("invokestatic java/lang/Double.toString(D)Ljava/lang/String;")
    def toString(d: scala.Double): String
    @js("Math.max($1, $2)")
    @jvm("invokestatic java/lang/Double.max(DD)D")
    def max(a: scala.Double, b: scala.Double): scala.Double
    @js("Math.min($1, $2)")
    @jvm("invokestatic java/lang/Double.min(DD)D")
    def min(a: scala.Double, b: scala.Double): scala.Double
    @js("$doubleHash($1)")
    @jvm("invokestatic java/lang/Double.hashCode(D)I")
    def hashCode(d: scala.Double): Int
    @js("$compareDoubles($1, $2)")
    @jvm("invokestatic java/lang/Double.compare(DD)I")
    def compare(a: scala.Double, b: scala.Double): Int
    @js("Number.isNaN($1)")
    @jvm("invokestatic java/lang/Double.isNaN(D)Z")
    def isNaN(d: scala.Double): Boolean
    @js("($1 === Infinity || $1 === -Infinity)")
    @jvm("invokestatic java/lang/Double.isInfinite(D)Z")
    def isInfinite(d: scala.Double): Boolean
    @js("Number.isFinite($1)")
    @jvm("invokestatic java/lang/Double.isFinite(D)Z")
    def isFinite(d: scala.Double): Boolean
    @js("$doubleToLongBits($1)")
    @jvm("invokestatic java/lang/Double.doubleToLongBits(D)J")
    def doubleToLongBits(d: scala.Double): scala.Long
    @js("$doubleToRawLongBits($1)")
    @jvm("invokestatic java/lang/Double.doubleToRawLongBits(D)J")
    def doubleToRawLongBits(d: scala.Double): scala.Long
    @js("$longBitsToDouble($1)")
    @jvm("invokestatic java/lang/Double.longBitsToDouble(J)D")
    def longBitsToDouble(l: scala.Long): scala.Double

  @javaDefined
  @jvmClass("java/lang/Float")
  final class Float private (value: scala.Float) extends Number, Comparable[Float]:
    @js("$d2i($0)")
    @jvm("invokevirtual java/lang/Float.intValue()I")
    def intValue: Int
    @js("$d2l($0)")
    @jvm("invokevirtual java/lang/Float.longValue()J")
    def longValue: scala.Long
    @js("$0")
    @jvm("invokevirtual java/lang/Float.floatValue()F")
    def floatValue: scala.Float
    @js("$0")
    @jvm("invokevirtual java/lang/Float.doubleValue()D")
    def doubleValue: scala.Double
    @js("$compareDoubles($0, $1)")
    @jvm("invokevirtual java/lang/Float.compareTo(Ljava/lang/Float;)I")
    def compareTo(that: Float): Int
    @js("Number.isNaN($0)")
    @jvm("invokevirtual java/lang/Float.isNaN()Z")
    def isNaN(): Boolean
    @js("($0 === Infinity || $0 === -Infinity)")
    @jvm("invokevirtual java/lang/Float.isInfinite()Z")
    def isInfinite(): Boolean

  @javaDefined
  @jvmClass("java/lang/Float")
  object Float:
    // `new Float(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Float dup $1:F invokespecial java/lang/Float.<init>(F)V")
    def newFloat(value: scala.Float): Float = Float.valueOf(value)
    @jvm("new java/lang/Float dup $1:D invokespecial java/lang/Float.<init>(D)V")
    def newFloat(value: scala.Double): Float = Float.valueOf(value.toFloat)
    @jvm("new java/lang/Float dup $1 invokespecial java/lang/Float.<init>(Ljava/lang/String;)V")
    def newFloat(s: String): Float = Float.valueOf(parseFloat(s))
    @jvm("invokestatic java/lang/Float.parseFloat(Ljava/lang/String;)F")
    def parseFloat(s: String): scala.Float = FloatingDecimal.parseFloat(s)
    // The JDK's constants, which scala-library's `scala.Float` reads.
    @js("3.4028234663852886e+38")
    @jvm("getstatic java/lang/Float.MAX_VALUE:F")
    def MAX_VALUE: scala.Float = intBitsToFloat(0x7f7fffff)
    @js("1.401298464324817e-45")
    @jvm("getstatic java/lang/Float.MIN_VALUE:F")
    def MIN_VALUE: scala.Float = intBitsToFloat(1)
    @js("1.1754943508222875e-38")
    @jvm("getstatic java/lang/Float.MIN_NORMAL:F")
    def MIN_NORMAL: scala.Float = intBitsToFloat(0x00800000)
    @js("Infinity")
    @jvm("getstatic java/lang/Float.POSITIVE_INFINITY:F")
    def POSITIVE_INFINITY: scala.Float = intBitsToFloat(0x7f800000)
    @js("-Infinity")
    @jvm("getstatic java/lang/Float.NEGATIVE_INFINITY:F")
    def NEGATIVE_INFINITY: scala.Float = intBitsToFloat(0xff800000)
    @js("NaN")
    @jvm("getstatic java/lang/Float.NaN:F")
    def NaN: scala.Float = intBitsToFloat(0x7fc00000)
    @jvm("getstatic java/lang/Float.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[scala.Float]
    @js("$1")
    @jvm("invokestatic java/lang/Float.valueOf(F)Ljava/lang/Float;")
    def valueOf(f: scala.Float): Float
    @js("Math.max($1, $2)")
    @jvm("invokestatic java/lang/Float.max(FF)F")
    def max(a: scala.Float, b: scala.Float): scala.Float
    @js("Math.min($1, $2)")
    @jvm("invokestatic java/lang/Float.min(FF)F")
    def min(a: scala.Float, b: scala.Float): scala.Float
    @js("Number.isNaN($1)")
    @jvm("invokestatic java/lang/Float.isNaN(F)Z")
    def isNaN(f: scala.Float): Boolean
    @js("(($1 === Infinity) || ($1 === -Infinity))")
    @jvm("invokestatic java/lang/Float.isInfinite(F)Z")
    def isInfinite(f: scala.Float): Boolean
    @js("Number.isFinite($1)")
    @jvm("invokestatic java/lang/Float.isFinite(F)Z")
    def isFinite(f: scala.Float): Boolean
    @js("$compareDoubles($1, $2)")
    @jvm("invokestatic java/lang/Float.compare(FF)I")
    def compare(a: scala.Float, b: scala.Float): Int
    @js("$floatHash($1)")
    @jvm("invokestatic java/lang/Float.hashCode(F)I")
    def hashCode(f: scala.Float): Int
    @js("$floatToIntBits($1)")
    @jvm("invokestatic java/lang/Float.floatToIntBits(F)I")
    def floatToIntBits(f: scala.Float): Int
    @js("$intBitsToFloat($1)")
    @jvm("invokestatic java/lang/Float.intBitsToFloat(I)F")
    def intBitsToFloat(i: Int): scala.Float
    @js("$floatToString($1)")
    @jvm("invokestatic java/lang/Float.toString(F)Ljava/lang/String;")
    def toString(f: scala.Float): String

  // The JDK's `jdk.internal.math.FloatingDecimal` for a `float`: `readJavaFormatString(s).floatValue()`.
  // The scan takes the decimal string's digits and exponent or goes to the hexadecimal one's
  // `parseHexString`; the decimal's `floatValue` rounds to the nearest float directly, a candidate
  // from float or double arithmetic corrected against the exact value in big integers, never through a
  // double. `BigInteger` stands for `FDBigInteger`.
  private[lang] object FloatingDecimal:
    private final val SINGLE_MAX_DECIMAL_DIGITS = 7
    private final val SINGLE_MAX_DECIMAL_EXPONENT = 38
    private final val SINGLE_MIN_DECIMAL_EXPONENT = -45
    private final val SINGLE_MAX_NDIGITS = 200
    private final val MAX_DECIMAL_DIGITS = 15
    private final val BIG_DECIMAL_EXPONENT = 324
    private final val SINGLE_EXP_SHIFT = 23
    private final val SINGLE_FRACT_HOB = 1 << 23
    private final val SIGNIF_BIT_MASK = 0x7FFFFF
    private final val EXP_BIAS = 127
    private final val EXP_BIT_MASK = 0x7F800000
    private final val SIGN_BIT_MASK = 0x80000000
    private val SMALL_10_POW: Array[scala.Double] = Array(
      1.0e0, 1.0e1, 1.0e2, 1.0e3, 1.0e4, 1.0e5, 1.0e6, 1.0e7, 1.0e8, 1.0e9, 1.0e10, 1.0e11,
      1.0e12, 1.0e13, 1.0e14, 1.0e15, 1.0e16, 1.0e17, 1.0e18, 1.0e19, 1.0e20, 1.0e21, 1.0e22)
    private val SINGLE_SMALL_10_POW: Array[scala.Float] = Array(
      1.0e0f, 1.0e1f, 1.0e2f, 1.0e3f, 1.0e4f, 1.0e5f, 1.0e6f, 1.0e7f, 1.0e8f, 1.0e9f, 1.0e10f)
    private val BIG_10_POW: Array[scala.Double] = Array(1e16, 1e32, 1e64, 1e128, 1e256)
    private val TINY_10_POW: Array[scala.Double] = Array(1e-16, 1e-32, 1e-64, 1e-128, 1e-256)
    private final val SINGLE_MAX_SMALL_TEN = 10

    def parseFloat(s: String): scala.Float = readJavaFormatString(s)

    private def signed(isNegative: scala.Boolean, f: scala.Float): scala.Float = if isNegative then -f else f

    private def readJavaFormatString(s: String): scala.Float =
      val in = s.trim
      val len = in.length
      if len == 0 then throw new NumberFormatException("empty String")
      def invalid: Nothing = throw new NumberFormatException("For input string: \"" + in + "\"")
      // `charAt` past the end, which the JDK's scan meets as a `StringIndexOutOfBoundsException`.
      def at(k: Int): Char = if k < len then in.charAt(k) else invalid
      var isNegative = false
      var signSeen = false
      var i = 0
      in.charAt(0) match
        case '-' =>
          isNegative = true
          i += 1
          signSeen = true
        case '+' =>
          i += 1
          signSeen = true
        case _ =>
      var c = at(i)
      if c == 'N' then
        if len - i == 3 && in.indexOf("NaN", i) == i then return scala.Float.NaN
        invalid
      if c == 'I' then
        if len - i == 8 && in.indexOf("Infinity", i) == i then
          return if isNegative then scala.Float.NegativeInfinity else scala.Float.PositiveInfinity
        invalid
      if c == '0' && len > i + 1 then
        val ch = in.charAt(i + 1)
        if ch == 'x' || ch == 'X' then return parseHexString(in)
      val digits = new Array[scala.Byte](len)
      var decSeen = false
      var nDigits = 0
      var decPt = 0
      var nLeadZero = 0
      var nTrailZero = 0
      var scanning = true
      while scanning && i < len do
        c = in.charAt(i)
        if c == '0' then nLeadZero += 1
        else if c == '.' then
          if decSeen then throw new NumberFormatException("multiple points")
          decPt = i
          if signSeen then decPt -= 1
          decSeen = true
        else scanning = false
        if scanning then i += 1
      scanning = true
      while scanning && i < len do
        c = in.charAt(i)
        if c >= '1' && c <= '9' then
          digits(nDigits) = c.toByte
          nDigits += 1
          nTrailZero = 0
        else if c == '0' then
          digits(nDigits) = c.toByte
          nDigits += 1
          nTrailZero += 1
        else if c == '.' then
          if decSeen then throw new NumberFormatException("multiple points")
          decPt = i
          if signSeen then decPt -= 1
          decSeen = true
        else scanning = false
        if scanning then i += 1
      nDigits -= nTrailZero
      val isZero = nDigits == 0
      if isZero && nLeadZero == 0 then invalid
      var decExp = if decSeen then decPt - nLeadZero else nDigits + nTrailZero
      if i < len && { c = in.charAt(i); c == 'e' || c == 'E' } then
        var expSign = 1
        var expVal = 0
        val reallyBig = Int.MaxValue / 10
        var expOverflow = false
        i += 1
        at(i) match
          case '-' =>
            expSign = -1
            i += 1
          case '+' => i += 1
          case _ =>
        val expAt = i
        scanning = true
        while scanning && i < len do
          if expVal >= reallyBig then expOverflow = true
          c = in.charAt(i)
          i += 1
          if c >= '0' && c <= '9' then expVal = expVal * 10 + (c - '0')
          else
            i -= 1
            scanning = false
        val expLimit = BIG_DECIMAL_EXPONENT + nDigits + nTrailZero
        if expOverflow || expVal > expLimit then
          if !expOverflow && (expSign == 1 && decExp < 0) && (expVal + decExp) < expLimit then decExp += expVal
          else decExp = expSign * expLimit
        else decExp = decExp + expSign * expVal
        if i == expAt then invalid
      if i < len && (i != len - 1 || (in.charAt(i) != 'f' && in.charAt(i) != 'F' && in.charAt(i) != 'd' && in.charAt(i) != 'D')) then
        invalid
      if isZero then return signed(isNegative, 0.0f)
      floatValue(isNegative, decExp, digits, nDigits)

    // `ASCIIToBinaryBuffer.floatValue`.
    private def floatValue(isNegative: scala.Boolean, decExponent: Int, digits: Array[scala.Byte], digitCount: Int): scala.Float =
      var nDigits = digitCount
      val kDigits = Math.min(nDigits, SINGLE_MAX_DECIMAL_DIGITS + 1)
      var iValue = digits(0) - '0'
      var k = 1
      while k < kDigits do
        iValue = iValue * 10 + digits(k) - '0'
        k += 1
      var fValue: scala.Float = iValue.toFloat
      var exp = decExponent - kDigits
      if nDigits <= SINGLE_MAX_DECIMAL_DIGITS then
        if exp == 0 || fValue == 0.0f then return signed(isNegative, fValue)
        else if exp >= 0 then
          if exp <= SINGLE_MAX_SMALL_TEN then
            fValue *= SINGLE_SMALL_10_POW(exp)
            return signed(isNegative, fValue)
          val slop = SINGLE_MAX_DECIMAL_DIGITS - kDigits
          if exp <= SINGLE_MAX_SMALL_TEN + slop then
            fValue *= SINGLE_SMALL_10_POW(slop)
            fValue *= SINGLE_SMALL_10_POW(exp - slop)
            return signed(isNegative, fValue)
        else if exp >= -SINGLE_MAX_SMALL_TEN then
          fValue /= SINGLE_SMALL_10_POW(-exp)
          return signed(isNegative, fValue)
      else if decExponent >= nDigits && nDigits + decExponent <= MAX_DECIMAL_DIGITS then
        var lValue = iValue.toLong
        k = kDigits
        while k < nDigits do
          lValue = lValue * 10L + (digits(k) - '0')
          k += 1
        var exact = lValue.toDouble
        exact *= SMALL_10_POW(decExponent - nDigits)
        return signed(isNegative, exact.toFloat)
      var dValue: scala.Double = fValue
      if exp > 0 then
        if decExponent > SINGLE_MAX_DECIMAL_EXPONENT + 1 then
          return if isNegative then scala.Float.NegativeInfinity else scala.Float.PositiveInfinity
        if (exp & 15) != 0 then dValue *= SMALL_10_POW(exp & 15)
        exp >>= 4
        var j = 0
        while exp > 0 do
          if (exp & 1) != 0 then dValue *= BIG_10_POW(j)
          j += 1
          exp >>= 1
      else if exp < 0 then
        exp = -exp
        if decExponent < SINGLE_MIN_DECIMAL_EXPONENT - 1 then return signed(isNegative, 0.0f)
        if (exp & 15) != 0 then dValue /= SMALL_10_POW(exp & 15)
        exp >>= 4
        var j = 0
        while exp > 0 do
          if (exp & 1) != 0 then dValue *= TINY_10_POW(j)
          j += 1
          exp >>= 1
      val approximate = dValue.toFloat
      fValue =
        if approximate < scala.Float.MinPositiveValue then scala.Float.MinPositiveValue
        else if approximate > scala.Float.MaxValue then scala.Float.MaxValue
        else approximate
      if nDigits > SINGLE_MAX_NDIGITS then
        nDigits = SINGLE_MAX_NDIGITS + 1
        digits(SINGLE_MAX_NDIGITS) = '1'.toByte
      val text = new java.lang.StringBuilder(nDigits)
      k = 0
      while k < nDigits do
        text.append(digits(k).toChar)
        k += 1
      exp = decExponent - nDigits
      var ieeeBits = Float.floatToIntBits(fValue)
      val B5 = Math.max(0, -exp)
      val D5 = Math.max(0, exp)
      val bigD0 = new java.math.BigInteger(text.toString).multiply(java.math.BigInteger.valueOf(5L).pow(D5))
      val fiveB5 = java.math.BigInteger.valueOf(5L).pow(B5)
      var bigD: java.math.BigInteger = null
      var prevD2 = 0
      var correcting = true
      while correcting do
        var binexp = ieeeBits >>> SINGLE_EXP_SHIFT
        var bigBbits = ieeeBits & SIGNIF_BIT_MASK
        if binexp > 0 then bigBbits |= SINGLE_FRACT_HOB
        else
          val shift = Integer.numberOfLeadingZeros(bigBbits) - (31 - SINGLE_EXP_SHIFT)
          bigBbits <<= shift
          binexp = 1 - shift
        binexp -= EXP_BIAS
        val lowOrderZeros = Integer.numberOfTrailingZeros(bigBbits)
        bigBbits >>>= lowOrderZeros
        val bigIntExp = binexp - SINGLE_EXP_SHIFT + lowOrderZeros
        val bigIntNBits = SINGLE_EXP_SHIFT + 1 - lowOrderZeros
        var B2 = B5
        var D2 = D5
        if bigIntExp >= 0 then B2 += bigIntExp else D2 -= bigIntExp
        var Ulp2 = B2
        val hulpbias = if binexp <= -EXP_BIAS then binexp + lowOrderZeros + EXP_BIAS else 1 + lowOrderZeros
        B2 += hulpbias
        D2 += hulpbias
        val common2 = Math.min(B2, Math.min(D2, Ulp2))
        B2 -= common2
        D2 -= common2
        Ulp2 -= common2
        val bigB = java.math.BigInteger.valueOf(bigBbits.toLong).multiply(fiveB5).shiftLeft(B2)
        if bigD == null || prevD2 != D2 then
          bigD = bigD0.shiftLeft(D2)
          prevD2 = D2
        val cmpResult = bigB.compareTo(bigD)
        if cmpResult == 0 then correcting = false
        else
          val overvalue = cmpResult > 0
          var diff = if overvalue then bigB.subtract(bigD) else bigD.subtract(bigB)
          if overvalue && bigIntNBits == 1 && bigIntExp > -EXP_BIAS + 1 then
            Ulp2 -= 1
            if Ulp2 < 0 then
              Ulp2 = 0
              diff = diff.shiftLeft(1)
          val halfUlp = diff.compareTo(fiveB5.shiftLeft(Ulp2))
          if halfUlp < 0 then correcting = false
          else if halfUlp == 0 then
            if (ieeeBits & 1) != 0 then ieeeBits += (if overvalue then -1 else 1)
            correcting = false
          else
            ieeeBits += (if overvalue then -1 else 1)
            if ieeeBits == 0 || ieeeBits == EXP_BIT_MASK then correcting = false
      if isNegative then ieeeBits |= SIGN_BIT_MASK
      Float.intBitsToFloat(ieeeBits)

    // `parseHexString`'s float: `HexFloatPattern.VALUE`'s grammar,
    // `([-+])?0[xX](((hex+)\.?)|((hex*)\.(hex+)))[pP]([-+])?(digit+)[fFdD]?`, scanned by hand
    // into its groups, the significand's bits copied with round and sticky bits and rounded to a
    // float's.
    private def parseHexString(s: String): scala.Float =
      def invalid: Nothing = throw new NumberFormatException("For input string: \"" + s + "\"")
      def isHex(c: Char): scala.Boolean = (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')
      val len = s.length
      var i = 0
      var group1: String = null
      if i < len && (s.charAt(i) == '-' || s.charAt(i) == '+') then
        group1 = s.substring(i, i + 1)
        i += 1
      if !(i + 1 < len && s.charAt(i) == '0' && (s.charAt(i + 1) == 'x' || s.charAt(i + 1) == 'X')) then invalid
      i += 2
      val intStart = i
      while i < len && isHex(s.charAt(i)) do i += 1
      val intDigits = s.substring(intStart, i)
      var group4: String = null
      var group6: String = null
      var group7: String = null
      if i < len && s.charAt(i) == '.' then
        i += 1
        val fracStart = i
        while i < len && isHex(s.charAt(i)) do i += 1
        val fracDigits = s.substring(fracStart, i)
        if fracDigits.isEmpty then
          if intDigits.isEmpty then invalid
          group4 = intDigits
        else
          group6 = intDigits
          group7 = fracDigits
      else
        if intDigits.isEmpty then invalid
        group4 = intDigits
      if !(i < len && (s.charAt(i) == 'p' || s.charAt(i) == 'P')) then invalid
      i += 1
      var group8: String = null
      if i < len && (s.charAt(i) == '-' || s.charAt(i) == '+') then
        group8 = s.substring(i, i + 1)
        i += 1
      val expStart = i
      while i < len && s.charAt(i) >= '0' && s.charAt(i) <= '9' do i += 1
      if i == expStart then invalid
      val group9 = s.substring(expStart, i)
      if i < len && (s.charAt(i) == 'f' || s.charAt(i) == 'F' || s.charAt(i) == 'd' || s.charAt(i) == 'D') then i += 1
      if i != len then invalid
      val isNegative = group1 != null && group1 == "-"
      var significandString: String = null
      var exponentAdjust = 0
      var leftDigits = 0
      var rightDigits = 0
      if group4 != null then
        significandString = stripLeadingZeros(group4)
        leftDigits = significandString.length
      else
        val g6 = stripLeadingZeros(group6)
        leftDigits = g6.length
        rightDigits = group7.length
        significandString = g6 + group7
      significandString = stripLeadingZeros(significandString)
      val signifLength = significandString.length
      exponentAdjust = if leftDigits >= 1 then 4 * (leftDigits - 1) else -4 * (rightDigits - signifLength + 1)
      if signifLength == 0 then return signed(isNegative, 0.0f)
      val positiveExponent = group8 == null || group8 == "+"
      val unsignedRawExponent: scala.Long =
        try Integer.parseInt(group9).toLong
        catch
          case _: NumberFormatException =>
            return if positiveExponent then (if isNegative then scala.Float.NegativeInfinity else scala.Float.PositiveInfinity)
            else signed(isNegative, 0.0f)
      val rawExponent = (if positiveExponent then 1L else -1L) * unsignedRawExponent
      var exponent = rawExponent + exponentAdjust
      var round = false
      var sticky = false
      var nextShift = 0
      var significand = 0L
      val leadingDigit = hexDigit(significandString, 0).toLong
      if leadingDigit == 1 then
        significand |= leadingDigit << 52
        nextShift = 52 - 4
      else if leadingDigit <= 3 then
        significand |= leadingDigit << 51
        nextShift = 52 - 5
        exponent += 1
      else if leadingDigit <= 7 then
        significand |= leadingDigit << 50
        nextShift = 52 - 6
        exponent += 2
      else
        significand |= leadingDigit << 49
        nextShift = 52 - 7
        exponent += 3
      var k = 1
      while k < signifLength && nextShift >= 0 do
        significand |= hexDigit(significandString, k).toLong << nextShift
        nextShift -= 4
        k += 1
      if k < signifLength then
        val currentDigit = hexDigit(significandString, k).toLong
        nextShift match
          case -1 =>
            significand |= (currentDigit & 0xEL) >> 1
            round = (currentDigit & 0x1L) != 0L
          case -2 =>
            significand |= (currentDigit & 0xCL) >> 2
            round = (currentDigit & 0x2L) != 0L
            sticky = (currentDigit & 0x1L) != 0
          case -3 =>
            significand |= (currentDigit & 0x8L) >> 3
            round = (currentDigit & 0x4L) != 0L
            sticky = (currentDigit & 0x3L) != 0
          case _ =>
            round = (currentDigit & 0x8L) != 0
            sticky = (currentDigit & 0x7L) != 0
        k += 1
        while k < signifLength && !sticky do
          sticky = hexDigit(significandString, k) != 0
          k += 1
      var floatBits = if isNegative then SIGN_BIT_MASK else 0
      if exponent >= -126 then
        if exponent > 127 then floatBits |= EXP_BIT_MASK
        else
          val threshShift = 53 - 24 - 1
          val floatSticky = (significand & ((1L << threshShift) - 1)) != 0 || round || sticky
          var iValue = (significand >>> threshShift).toInt
          if (iValue & 3) != 1 || floatSticky then iValue += 1
          floatBits |= ((exponent.toInt + (EXP_BIAS - 1)) << SINGLE_EXP_SHIFT) + (iValue >> 1)
      else if exponent >= -149 - 1 then
        val threshShift = ((53 - 2 + -149) - exponent).toInt
        val floatSticky = (significand & ((1L << threshShift) - 1)) != 0 || round || sticky
        var iValue = (significand >>> threshShift).toInt
        if (iValue & 3) != 1 || floatSticky then iValue += 1
        floatBits |= iValue >> 1
      if exponent > 1023 then return if isNegative then scala.Float.NegativeInfinity else scala.Float.PositiveInfinity
      Float.intBitsToFloat(floatBits)

    private def stripLeadingZeros(s: String): String =
      if !s.isEmpty && s.charAt(0) == '0' then
        var i = 1
        while i < s.length && s.charAt(i) == '0' do i += 1
        s.substring(i)
      else s

    private def hexDigit(s: String, position: Int): Int =
      val c = s.charAt(position)
      if c >= '0' && c <= '9' then c - '0' else if c >= 'a' && c <= 'f' then c - 'a' + 10 else c - 'A' + 10

  @javaDefined
  @jvmClass("java/lang/Boolean")
  final class Boolean private (value: scala.Boolean) extends Comparable[Boolean]:
    @js("$0")
    @jvm("invokevirtual java/lang/Boolean.booleanValue()Z")
    def booleanValue: scala.Boolean
    @js("($0 === $1 ? 0 : $0 ? 1 : -1)")
    @jvm("invokevirtual java/lang/Boolean.compareTo(Ljava/lang/Boolean;)I")
    def compareTo(that: Boolean): Int

  @javaDefined
  @jvmClass("java/lang/Boolean")
  object Boolean:
    // `new Boolean(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Boolean dup $1:Z invokespecial java/lang/Boolean.<init>(Z)V")
    def newBoolean(value: scala.Boolean): Boolean = Boolean.valueOf(value)
    @jvm("new java/lang/Boolean dup $1 invokespecial java/lang/Boolean.<init>(Ljava/lang/String;)V")
    def newBoolean(s: String): Boolean = Boolean.valueOf(parseBoolean(s))
    @jvm("getstatic java/lang/Boolean.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[scala.Boolean]
    @js("($1 === $2 ? 0 : $1 ? 1 : -1)")
    @jvm("invokestatic java/lang/Boolean.compare(ZZ)I")
    def compare(a: scala.Boolean, b: scala.Boolean): Int
    @js("($1 ? 1231 : 1237)")
    @jvm("invokestatic java/lang/Boolean.hashCode(Z)I")
    def hashCode(b: scala.Boolean): Int
    @js("(\"\" + $1)")
    @jvm("invokestatic java/lang/Boolean.toString(Z)Ljava/lang/String;")
    def toString(b: scala.Boolean): String
    @js("($1 !== null && $1.toLowerCase() === \"true\")")
    @jvm("invokestatic java/lang/Boolean.parseBoolean(Ljava/lang/String;)Z")
    def parseBoolean(s: String): scala.Boolean
    @js("$1")
    @jvm("invokestatic java/lang/Boolean.valueOf(Z)Ljava/lang/Boolean;")
    def valueOf(b: scala.Boolean): Boolean

  @javaDefined
  @jvmClass("java/lang/Character")
  final class Character private (value: Char) extends Comparable[Character]:
    @js("$0")
    @jvm("invokevirtual java/lang/Character.charValue()C")
    def charValue: Char
    @js("($charCode($0) - $charCode($1))")
    @jvm("invokevirtual java/lang/Character.compareTo(Ljava/lang/Character;)I")
    def compareTo(that: Character): Int

  @javaDefined
  @jvmClass("java/lang/Character")
  object Character:
    // `new Character(...)`, the JDK's deprecated constructors: the typer sends them here, as Scala.js
    // sends those of its hijacked classes to their companion (`JSCodeGen.genNewHijackedClass`).
    @js("$1")
    @jvm("new java/lang/Character dup $1:C invokespecial java/lang/Character.<init>(C)V")
    def newCharacter(value: Char): Character = Character.valueOf(value)
    @jvm("getstatic java/lang/Character.TYPE:Ljava/lang/Class;")
    def TYPE: Class[?] = classOf[Char]
    final val MAX_VALUE: '\uffff' = '\uffff'
    final val MIN_VALUE: '\u0000' = '\u0000'
    final val MIN_RADIX: 2 = 2
    final val MAX_RADIX: 36 = 36
    @js("$charCode($1)")
    @jvm("invokestatic java/lang/Character.hashCode(C)I")
    def hashCode(c: Char): Int
    @js("($charCode($1) - $charCode($2))")
    @jvm("invokestatic java/lang/Character.compare(CC)I")
    def compare(a: Char, b: Char): Int
    @js("/\\p{Nd}/u.test($1)")
    @jvm("invokestatic java/lang/Character.isDigit(C)Z")
    def isDigit(c: Char): scala.Boolean
    @js("/\\p{L}/u.test($1)")
    @jvm("invokestatic java/lang/Character.isLetter(C)Z")
    def isLetter(c: Char): scala.Boolean
    @js("/[\\p{L}\\p{Nd}]/u.test($1)")
    @jvm("invokestatic java/lang/Character.isLetterOrDigit(C)Z")
    def isLetterOrDigit(c: Char): scala.Boolean
    @js("/[\\p{L}\\p{Nl}\\p{Nd}\\p{Mn}\\p{Mc}\\p{Pc}\\p{Sc}\\p{Cf}\\x00-\\x08\\x0E-\\x1B\\x7F-\\x9F]/u.test($1)")
    @jvm("invokestatic java/lang/Character.isJavaIdentifierPart(C)Z")
    def isJavaIdentifierPart(c: Char): scala.Boolean
    @js("/\\p{ID_Start}/u.test($1)")
    @jvm("invokestatic java/lang/Character.isUnicodeIdentifierStart(C)Z")
    def isUnicodeIdentifierStart(c: Char): scala.Boolean
    @js("/[\\p{ID_Continue}\\p{Cf}\\x00-\\x08\\x0E-\\x1B\\x7F-\\x9F]/u.test($1)")
    @jvm("invokestatic java/lang/Character.isUnicodeIdentifierPart(C)Z")
    def isUnicodeIdentifierPart(c: Char): scala.Boolean
    @js("/\\s/.test($1)")
    @jvm("invokestatic java/lang/Character.isWhitespace(C)Z")
    def isWhitespace(c: Char): scala.Boolean
    @js("($1 !== $1.toLowerCase() && $1 === $1.toUpperCase())")
    @jvm("invokestatic java/lang/Character.isUpperCase(C)Z")
    def isUpperCase(c: Char): scala.Boolean
    @js("($1 !== $1.toUpperCase() && $1 === $1.toLowerCase())")
    @jvm("invokestatic java/lang/Character.isLowerCase(C)Z")
    def isLowerCase(c: Char): scala.Boolean
    @js("$1.toUpperCase()")
    @jvm("invokestatic java/lang/Character.toUpperCase(C)C")
    def toUpperCase(c: Char): Char
    @js("$1.toLowerCase()")
    @jvm("invokestatic java/lang/Character.toLowerCase(C)C")
    def toLowerCase(c: Char): Char
    @js("$digit($1, $2)")
    @jvm("invokestatic java/lang/Character.digit(CI)I")
    def digit(c: Char, radix: Int): Int
    @js("$1")
    @jvm("invokestatic java/lang/Character.toString(C)Ljava/lang/String;")
    def toString(c: Char): String
    @jvm("invokestatic java/lang/Character.forDigit(II)C")
    def forDigit(digit: Int, radix: Int): Char =
      if radix < 2 || radix > 36 || digit < 0 || digit >= radix then '\u0000'
      else if digit < 10 then ('0' + digit).toChar
      else ('a' + digit - 10).toChar
    @js("$numericValue($1)")
    @jvm("invokestatic java/lang/Character.getNumericValue(C)I")
    def getNumericValue(c: Char): Int
    @js("(($charCode($1) & 0xf800) === 0xd800)")
    @jvm("invokestatic java/lang/Character.isSurrogate(C)Z")
    def isSurrogate(c: Char): scala.Boolean
    @js("(($charCode($1) & 0xfc00) === 0xd800)")
    @jvm("invokestatic java/lang/Character.isHighSurrogate(C)Z")
    def isHighSurrogate(c: Char): scala.Boolean
    @js("(($charCode($1) & 0xfc00) === 0xdc00)")
    @jvm("invokestatic java/lang/Character.isLowSurrogate(C)Z")
    def isLowSurrogate(c: Char): scala.Boolean
    @js("$1")
    @jvm("invokestatic java/lang/Character.valueOf(C)Ljava/lang/Character;")
    def valueOf(c: Char): Character

  @javaDefined
  @jvmClass("java/lang/Math")
  final class Math private ()

  @javaDefined
  @jvmClass("java/lang/Math")
  object Math:
    final val PI: 3.141592653589793 = 3.141592653589793
    final val E: 2.718281828459045 = 2.718281828459045
    @js("$max($1, $2)")
    @jvm("rt $1:L $2:L rtcall numMax(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
    def max[T <: Int | scala.Long | scala.Float | scala.Double](a: T, b: T): T
    @js("$min($1, $2)")
    @jvm("rt $1:L $2:L rtcall numMin(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
    def min[T <: Int | scala.Long | scala.Float | scala.Double](a: T, b: T): T
    @js("$abs($1)")
    @jvm("rt $1:L rtcall numAbs(Ljava/lang/Object;)Ljava/lang/Object;")
    def abs[T <: Int | scala.Long | scala.Float | scala.Double](x: T): T
    @js("Math.sqrt($1)")
    @jvm("invokestatic java/lang/Math.sqrt(D)D")
    def sqrt(x: scala.Double): scala.Double
    @js("Math.cbrt($1)")
    @jvm("invokestatic java/lang/Math.cbrt(D)D")
    def cbrt(x: scala.Double): scala.Double
    @js("Math.pow($1, $2)")
    @jvm("invokestatic java/lang/Math.pow(DD)D")
    def pow(x: scala.Double, y: scala.Double): scala.Double
    @js("Math.floor($1)")
    @jvm("invokestatic java/lang/Math.floor(D)D")
    def floor(x: scala.Double): scala.Double
    @js("Math.ceil($1)")
    @jvm("invokestatic java/lang/Math.ceil(D)D")
    def ceil(x: scala.Double): scala.Double
    @js("$rint($1)")
    @jvm("invokestatic java/lang/Math.rint(D)D")
    def rint(x: scala.Double): scala.Double
    @js("$roundToLong($1)")
    @jvm("invokestatic java/lang/Math.round(D)J")
    def round(x: scala.Double): scala.Long
    @js("Math.exp($1)")
    @jvm("invokestatic java/lang/Math.exp(D)D")
    def exp(x: scala.Double): scala.Double
    @js("Math.log($1)")
    @jvm("invokestatic java/lang/Math.log(D)D")
    def log(x: scala.Double): scala.Double
    @js("Math.log10($1)")
    @jvm("invokestatic java/lang/Math.log10(D)D")
    def log10(x: scala.Double): scala.Double
    @js("Math.sin($1)")
    @jvm("invokestatic java/lang/Math.sin(D)D")
    def sin(x: scala.Double): scala.Double
    @js("Math.cos($1)")
    @jvm("invokestatic java/lang/Math.cos(D)D")
    def cos(x: scala.Double): scala.Double
    @js("Math.tan($1)")
    @jvm("invokestatic java/lang/Math.tan(D)D")
    def tan(x: scala.Double): scala.Double
    @js("Math.atan2($1, $2)")
    @jvm("invokestatic java/lang/Math.atan2(DD)D")
    def atan2(y: scala.Double, x: scala.Double): scala.Double
    @js("Math.hypot($1, $2)")
    @jvm("invokestatic java/lang/Math.hypot(DD)D")
    def hypot(x: scala.Double, y: scala.Double): scala.Double
    @js("$nextUp($1)")
    @jvm("invokestatic java/lang/Math.nextUp(D)D")
    def nextUp(d: scala.Double): scala.Double
    @js("$nextDown($1)")
    @jvm("invokestatic java/lang/Math.nextDown(D)D")
    def nextDown(d: scala.Double): scala.Double
    @js("Math.sign($1)")
    @jvm("invokestatic java/lang/Math.signum(D)D")
    def signum(x: scala.Double): scala.Double
    @js("$floorDivInt($1, $2)")
    @jvm("invokestatic java/lang/Math.floorDiv(II)I")
    def floorDiv(a: Int, b: Int): Int
    @js("$floorModInt($1, $2)")
    @jvm("invokestatic java/lang/Math.floorMod(II)I")
    def floorMod(a: Int, b: Int): Int
    @js("$floorDivLong($1, $2)")
    @jvm("invokestatic java/lang/Math.floorDiv(JJ)J")
    def floorDiv(a: scala.Long, b: scala.Long): scala.Long
    @js("$floorModLong($1, $2)")
    @jvm("invokestatic java/lang/Math.floorMod(JJ)J")
    def floorMod(a: scala.Long, b: scala.Long): scala.Long
    @js("$addExact($1, $2)")
    @jvm("invokestatic java/lang/Math.addExact(II)I")
    def addExact(a: Int, b: Int): Int
    @js("$multiplyExact($1, $2)")
    @jvm("invokestatic java/lang/Math.multiplyExact(II)I")
    def multiplyExact(a: Int, b: Int): Int
    @js("$toIntExact($1)")
    @jvm("invokestatic java/lang/Math.toIntExact(J)I")
    def toIntExact(l: scala.Long): Int
    @js("$subtractExact($1, $2)")
    @jvm("invokestatic java/lang/Math.subtractExact(II)I")
    def subtractExact(a: Int, b: Int): Int
    @js("$longExact($1 + $2)")
    @jvm("invokestatic java/lang/Math.addExact(JJ)J")
    def addExact(a: scala.Long, b: scala.Long): scala.Long
    @js("$longExact($1 - $2)")
    @jvm("invokestatic java/lang/Math.subtractExact(JJ)J")
    def subtractExact(a: scala.Long, b: scala.Long): scala.Long
    @js("$longExact($1 * $2)")
    @jvm("invokestatic java/lang/Math.multiplyExact(JJ)J")
    def multiplyExact(a: scala.Long, b: scala.Long): scala.Long
    @js("$longExact($1 * BigInt($2))")
    @jvm("invokestatic java/lang/Math.multiplyExact(JI)J")
    def multiplyExact(a: scala.Long, b: Int): scala.Long
    @js("$longExact(-$1)")
    @jvm("invokestatic java/lang/Math.negateExact(J)J")
    def negateExact(a: scala.Long): scala.Long
    @js("$subtractExact(0, $1)")
    @jvm("invokestatic java/lang/Math.negateExact(I)I")
    def negateExact(a: Int): Int
    @js("Number($floorModLong($1, BigInt($2)))")
    @jvm("invokestatic java/lang/Math.floorMod(JI)I")
    def floorMod(a: scala.Long, b: Int): Int
    @js("$floorDivLong($1, BigInt($2))")
    @jvm("invokestatic java/lang/Math.floorDiv(JI)J")
    def floorDiv(a: scala.Long, b: Int): scala.Long

  @javaDefined
  @jvmClass("java/lang/System")
  final class System private ()

  @javaDefined
  @jvmClass("java/lang/System")
  object System:
    @js("\"out\"")
    @jvm("getstatic java/lang/System.out:Ljava/io/PrintStream;")
    def out: java.io.PrintStream
    @js("\"err\"")
    @jvm("getstatic java/lang/System.err:Ljava/io/PrintStream;")
    def err: java.io.PrintStream
    // The interpreter's stdin (`java.io.Stdin`); JavaScript has none.
    @js("$fail(\"UnsupportedOperationException\", \"System.in is not available on JavaScript\")")
    @jvm("getstatic java/lang/System.in:Ljava/io/InputStream;")
    def in: java.io.InputStream
    @js("$arraycopy($1, $2, $3, $4, $5)")
    @jvm("$1:L $2:I $3:L $4:I $5:I invokestatic java/lang/System.arraycopy(Ljava/lang/Object;ILjava/lang/Object;II)V")
    def arraycopy(src: AnyRef, srcPos: Int, dest: AnyRef, destPos: Int, length: Int): Unit
    @js("$identityHash($1)")
    @jvm("invokestatic java/lang/System.identityHashCode(Ljava/lang/Object;)I")
    def identityHashCode(x: AnyRef): Int
    @js("$toLong(Date.now())")
    @jvm("invokestatic java/lang/System.currentTimeMillis()J")
    def currentTimeMillis(): scala.Long
    @js("$toLong(Math.round(performance.now() * 1000000))")
    @jvm("invokestatic java/lang/System.nanoTime()J")
    def nanoTime(): scala.Long
    @js("\"\\n\"")
    @jvm("invokestatic java/lang/System.lineSeparator()Ljava/lang/String;")
    def lineSeparator(): String
    // Scala.js answers null for a property outside its small built-in set.
    @js("null")
    @jvm("invokestatic java/lang/System.getProperty(Ljava/lang/String;)Ljava/lang/String;")
    def getProperty(key: String): String
    @js("$2")
    @jvm("invokestatic java/lang/System.getProperty(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;")
    def getProperty(key: String, default: String): String
    // Scala.js has no environment: every variable is unset (munit reads `NO_COLOR` so).
    @js("null")
    @jvm("invokestatic java/lang/System.getenv(Ljava/lang/String;)Ljava/lang/String;")
    def getenv(key: String): String
    // scala-library's `sys.env`, which a macro reads (levsha's optimizer): empty on JavaScript,
    // as Scala.js has it.
    @jvm("invokestatic java/lang/System.getenv()Ljava/util/Map;")
    def getenv(): java.util.Map[String, String] = environment()
    private[lang] def environment(): java.util.Map[String, String] =
      val env = new java.util.HashMap[String, String]()
      val entries = envEntries()
      var i = 0
      while i + 1 < entries.length do
        env.put(entries(i), entries(i + 1))
        i += 2
      env
    // The names and values, alternating.
    @js("[]")
    private def envEntries(): Array[String]
    // The end of the program with its status, as the JVM halts: nothing after it runs, a `finally`
    // included. A JavaScript program has no process of its own to end.
    @js("$fail(\"UnsupportedOperationException\", \"System.exit is not available on JavaScript\")")
    @jvm("invokestatic java/lang/System.exit(I)V")
    def exit(status: Int): Unit

  /// `java.lang.StringBuilder` over a JavaScript string; `append` returns the builder.
  @jvmClass("java/lang/StringBuilder")
  final class StringBuilder(private var s: String) extends CharSequence, Appendable:
    def this() = this("")
    def this(capacity: Int) = this("")
    @jvm("invokevirtual java/lang/StringBuilder.append(Ljava/lang/Object;)Ljava/lang/StringBuilder;")
    def append(x: Any): StringBuilder =
      s = s + x
      this
    @jvm("invokevirtual java/lang/StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;")
    def append(x: String): StringBuilder =
      s = s + x
      this
    @jvm("invokevirtual java/lang/StringBuilder.append(C)Ljava/lang/StringBuilder;")
    def append(x: Char): StringBuilder =
      s = s + x
      this
    @jvm("invokevirtual java/lang/StringBuilder.append(I)Ljava/lang/StringBuilder;")
    def append(x: Int): StringBuilder =
      s = s + x
      this
    // The characters from `start` to `end` (scala-library's `StringOps.patch` appends through it).
    @jvm("invokevirtual java/lang/StringBuilder.append(Ljava/lang/CharSequence;II)Ljava/lang/StringBuilder;")
    def append(x: CharSequence, start: Int, end: Int): StringBuilder =
      val text = if x == null then "null" else x.toString
      if start < 0 || start > end || end > text.length then throw new IndexOutOfBoundsException("start " + start + ", end " + end + ", length " + text.length)
      s = s + text.substring(start, end)
      this
    @jvm("invokevirtual java/lang/StringBuilder.append([CII)Ljava/lang/StringBuilder;")
    def append(x: Array[Char], offset: Int, len: Int): StringBuilder =
      val end = offset + len
      if offset < 0 || offset > end || end > x.length then
        throw new IndexOutOfBoundsException(s"Range [$offset, $end) out of bounds for length ${x.length}")
      var i = offset
      while i < end do
        s = s + x(i)
        i += 1
      this
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.length()I")
    def length(): Int = s.length
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.charAt(I)C")
    def charAt(i: Int): Char = s.charAt(i)
    @jvm("invokevirtual java/lang/StringBuilder.setLength(I)V")
    def setLength(n: Int): Unit =
      if n <= s.length then s = s.substring(0, n)
      else while s.length < n do s = s + "\u0000"
    @jvm("invokevirtual java/lang/StringBuilder.insert(ILjava/lang/String;)Ljava/lang/StringBuilder;")
    def insert(at: Int, x: String): StringBuilder =
      s = s.substring(0, at) + x + s.substring(at)
      this
    @jvm("invokevirtual java/lang/StringBuilder.insert(IC)Ljava/lang/StringBuilder;")
    def insert(at: Int, c: Char): StringBuilder = insert(at, c.toString)
    @jvm("invokevirtual java/lang/StringBuilder.insert(ILjava/lang/Object;)Ljava/lang/StringBuilder;")
    def insert(at: Int, x: Any): StringBuilder = insert(at, "" + x)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.reverse()Ljava/lang/StringBuilder;")
    def reverse(): StringBuilder =
      s = s.reverse
      this
    @jvm("invokevirtual java/lang/StringBuilder.ensureCapacity(I)V")
    def ensureCapacity(n: Int): Unit = ()
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.capacity()I")
    def capacity(): Int = s.length
    @jvm("invokevirtual java/lang/StringBuilder.getChars(II[CI)V")
    def getChars(srcBegin: Int, srcEnd: Int, dst: Array[Char], dstBegin: Int): Unit =
      var i = srcBegin
      while i < srcEnd do
        dst(dstBegin + i - srcBegin) = s.charAt(i)
        i += 1
    @jvm("invokevirtual java/lang/StringBuilder.setCharAt(IC)V")
    def setCharAt(i: Int, c: Char): Unit = s = s.substring(0, i) + c + s.substring(i + 1)
    @jvm("invokevirtual java/lang/StringBuilder.deleteCharAt(I)Ljava/lang/StringBuilder;")
    def deleteCharAt(i: Int): StringBuilder =
      s = s.substring(0, i) + s.substring(i + 1)
      this
    @jvm("invokevirtual java/lang/StringBuilder.delete(II)Ljava/lang/StringBuilder;")
    def delete(start: Int, end: Int): StringBuilder =
      s = s.substring(0, start) + s.substring(if end > s.length then s.length else end)
      this
    @jvm("invokevirtual java/lang/StringBuilder.replace(IILjava/lang/String;)Ljava/lang/StringBuilder;")
    def replace(start: Int, end: Int, str: String): StringBuilder =
      s = s.substring(0, start) + str + s.substring(if end > s.length then s.length else end)
      this
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.indexOf(Ljava/lang/String;)I")
    def indexOf(str: String): Int = s.indexOf(str)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.indexOf(Ljava/lang/String;I)I")
    def indexOf(str: String, from: Int): Int = s.indexOf(str, from)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.lastIndexOf(Ljava/lang/String;)I")
    def lastIndexOf(str: String): Int = s.lastIndexOf(str)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.substring(I)Ljava/lang/String;")
    def substring(start: Int): String = s.substring(start)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.substring(II)Ljava/lang/String;")
    def substring(start: Int, end: Int): String = s.substring(start, end)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.subSequence(II)Ljava/lang/CharSequence;")
    def subSequence(start: Int, end: Int): CharSequence = s.substring(start, end)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuilder.isEmpty()Z")
    def isEmpty(): scala.Boolean = s.length == 0
    @jvm("invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String;")
    override def toString: String = s

  /// `java.lang.StringBuffer`: what `StringBuilder` is, under the JDK's synchronised name.
  @jvmClass("java/lang/StringBuffer")
  final class StringBuffer(private var s: String) extends CharSequence, Appendable:
    def this() = this("")
    def this(capacity: Int) = this("")
    @jvm("invokevirtual java/lang/StringBuffer.append(Ljava/lang/Object;)Ljava/lang/StringBuffer;")
    def append(x: Any): StringBuffer =
      s = s + x
      this
    @jvm("invokevirtual java/lang/StringBuffer.append(Ljava/lang/String;)Ljava/lang/StringBuffer;")
    def append(x: String): StringBuffer =
      s = s + x
      this
    @jvm("invokevirtual java/lang/StringBuffer.append(C)Ljava/lang/StringBuffer;")
    def append(x: Char): StringBuffer =
      s = s + x
      this
    @jvm("invokevirtual java/lang/StringBuffer.append(I)Ljava/lang/StringBuffer;")
    def append(x: Int): StringBuffer =
      s = s + x
      this
    @jvm("invokevirtual java/lang/StringBuffer.appendCodePoint(I)Ljava/lang/StringBuffer;")
    def appendCodePoint(cp: Int): StringBuffer =
      s = s + Character.toChars(cp).mkString
      this
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.length()I")
    def length(): Int = s.length
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.charAt(I)C")
    def charAt(i: Int): Char = s.charAt(i)
    @jvm("invokevirtual java/lang/StringBuffer.setLength(I)V")
    def setLength(n: Int): Unit =
      if n <= s.length then s = s.substring(0, n)
      else while s.length < n do s = s + "\u0000"
    @jvm("invokevirtual java/lang/StringBuffer.setCharAt(IC)V")
    def setCharAt(i: Int, c: Char): Unit = s = s.substring(0, i) + c + s.substring(i + 1)
    @jvm("invokevirtual java/lang/StringBuffer.insert(ILjava/lang/String;)Ljava/lang/StringBuffer;")
    def insert(at: Int, x: String): StringBuffer =
      s = s.substring(0, at) + x + s.substring(at)
      this
    @jvm("invokevirtual java/lang/StringBuffer.insert(IC)Ljava/lang/StringBuffer;")
    def insert(at: Int, c: Char): StringBuffer = insert(at, c.toString)
    @jvm("invokevirtual java/lang/StringBuffer.deleteCharAt(I)Ljava/lang/StringBuffer;")
    def deleteCharAt(i: Int): StringBuffer =
      s = s.substring(0, i) + s.substring(i + 1)
      this
    @jvm("invokevirtual java/lang/StringBuffer.delete(II)Ljava/lang/StringBuffer;")
    def delete(start: Int, end: Int): StringBuffer =
      s = s.substring(0, start) + s.substring(if end > s.length then s.length else end)
      this
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.reverse()Ljava/lang/StringBuffer;")
    def reverse(): StringBuffer =
      s = s.reverse
      this
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.indexOf(Ljava/lang/String;)I")
    def indexOf(str: String): Int = s.indexOf(str)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.substring(I)Ljava/lang/String;")
    def substring(start: Int): String = s.substring(start)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.substring(II)Ljava/lang/String;")
    def substring(start: Int, end: Int): String = s.substring(start, end)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.subSequence(II)Ljava/lang/CharSequence;")
    def subSequence(start: Int, end: Int): CharSequence = s.substring(start, end)
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.isEmpty()Z")
    def isEmpty(): scala.Boolean = s.length == 0
    @javaDefined
    @jvm("invokevirtual java/lang/StringBuffer.capacity()I")
    def capacity(): Int = s.length
    @jvm("invokevirtual java/lang/StringBuffer.ensureCapacity(I)V")
    def ensureCapacity(n: Int): Unit = ()
    @jvm("invokevirtual java/lang/StringBuffer.toString()Ljava/lang/String;")
    override def toString: String = s

  // A thread-local is one slot on JavaScript, which has one thread; `DynamicVariable`, and
  // through it `Console`, keeps its value in one.
  @javaDefined
  @jvmClass("java/lang/ThreadLocal")
  class ThreadLocal[T]:
    private var value: T = null.asInstanceOf[T]
    private var present: scala.Boolean = false
    protected def initialValue(): T = null.asInstanceOf[T]
    def get(): T =
      if !present then set(initialValue())
      value
    def set(v: T): Unit =
      value = v
      present = true
    def remove(): Unit =
      value = null.asInstanceOf[T]
      present = false

  @javaDefined
  @jvmClass("java/lang/InheritableThreadLocal")
  class InheritableThreadLocal[T] extends ThreadLocal[T]:
    protected def childValue(parentValue: T): T = parentValue

  // What a test framework is handed for loading its suites, which on JavaScript loads nothing
  // (Scala.js's javalib has the class alone; `scala.scalajs.reflect` finds the classes).
  @javaDefined
  @jvmClass("java/lang/ClassLoader")
  abstract class ClassLoader protected (parent: ClassLoader):
    protected def this() = this(null)
    final def getParent(): ClassLoader = parent

package java.lang.ref:
  // Nothing collects a referent on JavaScript: a reference holds it until cleared.
  @javaDefined
  @jvmClass("java/lang/ref/Reference")
  abstract class Reference[T](private var referent: T):
    def get(): T = referent
    def clear(): Unit = referent = null.asInstanceOf[T]
    def refersTo(obj: T): scala.Boolean = referent == obj
    def enqueue(): scala.Boolean = false

  @javaDefined
  @jvmClass("java/lang/ref/WeakReference")
  class WeakReference[T](referent: T) extends Reference[T](referent)

  @javaDefined
  @jvmClass("java/lang/ref/SoftReference")
  class SoftReference[T](referent: T) extends Reference[T](referent)

package java.lang.annotation:
  // The interface an annotation type implements, which a class may extend as well (munit's
  // `Location` does).
  @javaDefined
  @jvmClass("java/lang/annotation/Annotation")
  trait Annotation:
    def annotationType(): Class[? <: Annotation]

package java.lang.reflect:
  // A method `Class.getMethod` found, invoked by name on the receiver in the interpreter.
  final class Method(name: String):
    def getName: String = name
    def invoke(obj: Any, args: Any*): AnyRef = invokeByName(obj, name, args.toList)
    override def toString: String = "public method " + name

  @js("$fail(\"UnsupportedOperationException\", \"reflection is not available on JavaScript\")")
  def invokeByName(obj: Any, name: String, args: List[Any]): AnyRef

  @javaDefined
  @jvmClass("java/lang/reflect/Array")
  object Array:
    @js("new Array($2).fill(null)")
    @jvm("invokestatic java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;")
    def newInstance(componentType: Class[?], length: Int): AnyRef

package scala.math:
  // The Java class behind `BigInt` and `BigDecimal`.
  @javaDefined
  @jvmClass("scala/math/ScalaNumber")
  abstract class ScalaNumber extends java.lang.Number:
    def underlying(): AnyRef
    def isWhole: scala.Boolean

package scala.runtime:
  // The Java classes of scala-library that its Scala code calls: the array allocation behind a
  // `ClassTag`, the hashing and the memory fence after a constructor; the hash mixing follows
  // the Java definitions.
  /** The box of `()`, which `classOf[Unit]` maps to (`classOf[BoxedUnit]` in tapir's table
    * of boxed classes); on JavaScript `()` is `undefined`, so no instance is made. */
  @javaDefined
  @jvmClass("scala/runtime/BoxedUnit")
  final class BoxedUnit private ():
    override def toString: String = "()"
    override def hashCode: Int = 0

  @javaDefined
  @jvmClass("scala/runtime/BoxedUnit")
  object BoxedUnit:
    @js("undefined")
    @jvm("getstatic scala/runtime/BoxedUnit.UNIT:Lscala/runtime/BoxedUnit;")
    def UNIT: BoxedUnit
    @js("$classNamed(\"scala.runtime.BoxedUnit\")")
    @jvm("getstatic scala/runtime/BoxedUnit.TYPE:Ljava/lang/Class;")
    def TYPE: java.lang.Class[?]

  @javaDefined
  @jvmClass("scala/runtime/Arrays")
  object Arrays:
    @jvm("invokestatic scala/runtime/Arrays.newGenericArray(ILscala/reflect/ClassTag;)Ljava/lang/Object;")
    def newGenericArray[T](length: Int)(implicit tag: scala.reflect.ClassTag[T]): scala.Array[T] = tag.newArray(length)

  @javaDefined
  @jvmClass("scala/runtime/Statics")
  object Statics:
    @jvm("invokestatic scala/runtime/Statics.releaseFence()V")
    def releaseFence(): Unit = ()
    @js("$hash($1)")
    @jvm("invokestatic scala/runtime/Statics.anyHash(Ljava/lang/Object;)I")
    def anyHash(x: Any): Int
    @jvm("invokestatic scala/runtime/Statics.mix(II)I")
    def mix(hash: Int, data: Int): Int =
      val h = java.lang.Integer.rotateLeft(mixLast(hash, data), 13)
      h * 5 + 0xe6546b64
    @jvm("invokestatic scala/runtime/Statics.mixLast(II)I")
    def mixLast(hash: Int, data: Int): Int =
      var k = data * 0xcc9e2d51
      k = java.lang.Integer.rotateLeft(k, 15)
      k = k * 0x1b873593
      hash ^ k
    @jvm("invokestatic scala/runtime/Statics.finalizeHash(II)I")
    def finalizeHash(hash: Int, length: Int): Int = avalanche(hash ^ length)
    @jvm("invokestatic scala/runtime/Statics.avalanche(I)I")
    def avalanche(hash: Int): Int =
      var h = hash
      h ^= h >>> 16
      h *= 0x85ebca6b
      h ^= h >>> 13
      h *= 0xc2b2ae35
      h ^= h >>> 16
      h
    @js("$longHash($1)")
    @jvm("invokestatic scala/runtime/Statics.longHash(J)I")
    def longHash(l: scala.Long): Int
    @js("$hash($1)")
    @jvm("invokestatic scala/runtime/Statics.doubleHash(D)I")
    def doubleHash(d: scala.Double): Int
    @js("$hash($1)")
    @jvm("invokestatic scala/runtime/Statics.floatHash(F)I")
    def floatHash(f: scala.Float): Int
    @jvm("invokestatic scala/runtime/Statics.ioobe(I)Ljava/lang/Object;")
    def ioobe[T](n: Int): T = throw new IndexOutOfBoundsException(n.toString)
    private class Marker
    @jvm("getstatic scala/runtime/Statics.pfMarker:Ljava/lang/Object;")
    val pfMarker: AnyRef = new Marker

// The boxing conversions of Predef: on JavaScript a boxed value is the value itself. A null box
// unboxes to the zero of its type, as the `x.asInstanceOf[Int]` of scalac's Predef does.
package scala:
  @predef
  implicit def int2Integer(x: Int): java.lang.Integer = java.lang.Integer.valueOf(x)
  @predef
  implicit def Integer2int(x: java.lang.Integer): Int = if x eq null then 0 else x.intValue
  @predef
  implicit def long2Long(x: Long): java.lang.Long = java.lang.Long.valueOf(x)
  @predef
  implicit def Long2long(x: java.lang.Long): Long = if x eq null then 0L else x.longValue
  @predef
  implicit def double2Double(x: Double): java.lang.Double = java.lang.Double.valueOf(x)
  @predef
  implicit def Double2double(x: java.lang.Double): Double = if x eq null then 0.0 else x.doubleValue
  @predef
  implicit def float2Float(x: Float): java.lang.Float = java.lang.Float.valueOf(x)
  @predef
  implicit def Float2float(x: java.lang.Float): Float = if x eq null then 0.0f else x.floatValue
  @predef
  implicit def boolean2Boolean(x: Boolean): java.lang.Boolean = java.lang.Boolean.valueOf(x)
  @predef
  implicit def Boolean2boolean(x: java.lang.Boolean): Boolean = if x eq null then false else x.booleanValue
  @predef
  implicit def char2Character(x: Char): java.lang.Character = java.lang.Character.valueOf(x)
  @predef
  implicit def Character2char(x: java.lang.Character): Char = if x eq null then '\u0000' else x.charValue
  @predef
  implicit def short2Short(x: Short): java.lang.Short = java.lang.Short.valueOf(x)
  @predef
  implicit def Short2short(x: java.lang.Short): Short = if x eq null then (0: Short) else x.shortValue
  @predef
  implicit def byte2Byte(x: Byte): java.lang.Byte = java.lang.Byte.valueOf(x)
  @predef
  implicit def Byte2byte(x: java.lang.Byte): Byte = if x eq null then (0: Byte) else x.byteValue
