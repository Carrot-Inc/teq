package scala.util.hashing

def byteswap32(v: Int): Int =
  val hc = java.lang.Integer.reverseBytes(v * 0x9e3775cd)
  hc * 0x9e3775cd

def byteswap64(v: Long): Long =
  val m = v * 0x9e3775cd9e3775cdL
  val hi = java.lang.Integer.reverseBytes(m.toInt).toLong << 32
  val lo = java.lang.Integer.reverseBytes((m >>> 32).toInt).toLong & 0xffffffffL
  (hi | lo) * 0x9e3775cd9e3775cdL

// scala-library's MurmurHash3 as a library body names it (cats' `Hash` instances); the
// collections hash through the runtime's copies of the same functions.
object MurmurHash3:
  final val arraySeed: Int = 0x3c074a61
  final val stringSeed: Int = 0xf7ca7fd2
  final val productSeed: Int = 0xcafebabe
  final val symmetricSeed: Int = 0xb592f7ae
  final val traversableSeed: Int = 0xe73a8b15
  final val seqSeed: Int = "Seq".hashCode
  final val mapSeed: Int = "Map".hashCode
  final val setSeed: Int = "Set".hashCode

  @js("$mix($1, $2)")
  def mix(hash: Int, data: Int): Int = scala.runtime.mix(hash, data)
  @js("$mixLast($1, $2)")
  def mixLast(hash: Int, data: Int): Int = scala.runtime.mixLast(hash, data)
  @js("$finalizeHash($1, $2)")
  def finalizeHash(hash: Int, length: Int): Int = scala.runtime.finalizeHash(hash, length)
  @js("$avalanche($1)")
  private def avalanche(hash: Int): Int = scala.runtime.avalanche(hash)

  def tuple2Hash(x: Any, y: Any): Int = tuple2Hash(x.##, y.##, productSeed)
  def tuple2Hash(x: Int, y: Int, seed: Int): Int =
    var h = seed
    h = mix(h, "Tuple2".hashCode)
    h = mix(h, x)
    h = mix(h, y)
    finalizeHash(h, 2)

  def productHash(x: Product, seed: Int, ignorePrefix: Boolean = false): Int =
    val arr = x.productArity
    if arr == 0 then (if !ignorePrefix then x.productPrefix.hashCode else seed)
    else
      var h = seed
      if !ignorePrefix then h = mix(h, x.productPrefix.hashCode)
      var i = 0
      while i < arr do
        h = mix(h, x.productElement(i).##)
        i += 1
      finalizeHash(h, arr)
  def productHash(x: Product): Int = caseClassHash(x, productSeed, null)

  def caseClassHash(x: Product, seed: Int, caseClassName: String | Null): Int =
    val arr = x.productArity
    val aye = (if caseClassName != null then caseClassName else x.productPrefix).hashCode
    if arr == 0 then aye
    else
      var h = seed
      h = mix(h, aye)
      var i = 0
      while i < arr do
        h = mix(h, x.productElement(i).##)
        i += 1
      finalizeHash(h, arr)
  def caseClassHash(x: Product, caseClassName: String | Null = null): Int = caseClassHash(x, productSeed, caseClassName)

  def stringHash(str: String, seed: Int): Int =
    var h = seed
    var i = 0
    while i + 1 < str.length do
      val data = (str.charAt(i) << 16) + str.charAt(i + 1)
      h = mix(h, data)
      i += 2
    if i < str.length then h = mixLast(h, str.charAt(i).toInt)
    finalizeHash(h, str.length)
  def stringHash(x: String): Int = stringHash(x, stringSeed)

  def unorderedHash(xs: IterableOnce[Any], seed: Int): Int =
    var a = 0
    var b = 0
    var n = 0
    var c = 1
    val iterator = xs.iterator
    while iterator.hasNext do
      val h = iterator.next().##
      a += h
      b ^= h
      c *= h | 1
      n += 1
    var h = seed
    h = mix(h, a)
    h = mix(h, b)
    h = mixLast(h, c)
    finalizeHash(h, n)
  def unorderedHash(xs: IterableOnce[Any]): Int = unorderedHash(xs, traversableSeed)

  // Elements whose hashes form an arithmetic progression hash as the range they could be.
  def orderedHash(xs: IterableOnce[Any], seed: Int): Int =
    val it = xs.iterator
    var h = seed
    if !it.hasNext then return finalizeHash(h, 0)
    val x0 = it.next()
    if !it.hasNext then return finalizeHash(mix(h, x0.##), 1)
    val x1 = it.next()
    val initial = x0.##
    h = mix(h, initial)
    val h0 = h
    var prev = x1.##
    val rangeDiff = prev - initial
    var i = 2
    while it.hasNext do
      h = mix(h, prev)
      val hash = it.next().##
      if rangeDiff != hash - prev || rangeDiff == 0 then
        h = mix(h, hash)
        i += 1
        while it.hasNext do
          h = mix(h, it.next().##)
          i += 1
        return finalizeHash(h, i)
      prev = hash
      i += 1
    avalanche(mix(mix(h0, rangeDiff), prev))
  def orderedHash(xs: IterableOnce[Any]): Int = orderedHash(xs, symmetricSeed)

  def rangeHash(start: Int, step: Int, last: Int, seed: Int): Int =
    avalanche(mix(mix(mix(seed, start), step), last))
  def rangeHash(start: Int, step: Int, last: Int): Int = rangeHash(start, step, last, seqSeed)

  def indexedSeqHash(a: IndexedSeq[Any], seed: Int): Int =
    var h = seed
    val l = a.length
    if l == 0 then finalizeHash(h, 0)
    else if l == 1 then finalizeHash(mix(h, a(0).##), 1)
    else
      val initial = a(0).##
      h = mix(h, initial)
      val h0 = h
      var prev = a(1).##
      val rangeDiff = prev - initial
      var i = 2
      while i < l do
        h = mix(h, prev)
        val hash = a(i).##
        if rangeDiff != hash - prev || rangeDiff == 0 then
          h = mix(h, hash)
          i += 1
          while i < l do
            h = mix(h, a(i).##)
            i += 1
          return finalizeHash(h, l)
        prev = hash
        i += 1
      avalanche(mix(mix(h0, rangeDiff), prev))

  def listHash(xs: List[?], seed: Int): Int =
    var n = 0
    var h = seed
    var rangeState = 0
    var rangeDiff = 0
    var prev = 0
    var initial = 0
    var elems = xs
    while !elems.isEmpty do
      val hash = elems.head.##
      h = mix(h, hash)
      if rangeState == 0 then
        initial = hash
        rangeState = 1
      else if rangeState == 1 then
        rangeDiff = hash - prev
        rangeState = 2
      else if rangeState == 2 && (rangeDiff != hash - prev || rangeDiff == 0) then rangeState = 3
      prev = hash
      n += 1
      elems = elems.tail
    if rangeState == 2 then rangeHash(initial, rangeDiff, prev, seed) else finalizeHash(h, n)

  def arrayHash[T](a: Array[T], seed: Int): Int = orderedHash(a.toIndexedSeq, seed)
  def arrayHash[T](a: Array[T]): Int = arrayHash(a, arraySeed)
  def bytesHash(data: Array[Byte], seed: Int): Int =
    var len = data.length
    var h = seed
    var i = 0
    while len >= 4 do
      var k = data(i + 0) & 0xFF
      k |= (data(i + 1) & 0xFF) << 8
      k |= (data(i + 2) & 0xFF) << 16
      k |= (data(i + 3) & 0xFF) << 24
      h = mix(h, k)
      i += 4
      len -= 4
    var k = 0
    if len == 3 then k ^= (data(i + 2) & 0xFF) << 16
    if len >= 2 then k ^= (data(i + 1) & 0xFF) << 8
    if len >= 1 then
      k ^= (data(i + 0) & 0xFF)
      h = mixLast(h, k)
    finalizeHash(h, data.length)
  def bytesHash(data: Array[Byte]): Int = bytesHash(data, arraySeed)
  def seqHash(xs: Seq[?]): Int = xs match
    case xs: IndexedSeq[?] => indexedSeqHash(xs, seqSeed)
    case xs: List[?] => listHash(xs, seqSeed)
    case xs => orderedHash(xs, seqSeed)
  def setHash(xs: Set[?]): Int = unorderedHash(xs, setSeed)
  def mapHash(xs: Map[?, ?]): Int =
    if xs.isEmpty then emptyMapHash
    else
      var a = 0
      var b = 0
      var n = 0
      var c = 1
      xs.foreach: (k, v) =>
        val h = tuple2Hash(k, v)
        a += h
        b ^= h
        c *= h | 1
        n += 1
      var h = mapSeed
      h = mix(h, a)
      h = mix(h, b)
      h = mixLast(h, c)
      finalizeHash(h, n)
  val emptyMapHash: Int = unorderedHash(Nil, mapSeed)
