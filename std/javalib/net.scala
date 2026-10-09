// The Java platform layer for JavaScript, `java.net`. JavaScript only: the JVM has the JDK's.
// `URLEncoder` as Scala.js's javalib has it, the JDK's `application/x-www-form-urlencoded`
// encoding but for a run of characters to encode that holds a lone surrogate: Scala.js keeps the
// bytes before it and drops the rest of the run where the JDK writes `?`. `URI` adapted from
// Scala.js's javalib (https://www.scala-js.org/, Copyright EPFL, Apache License 2.0), which parses
// by RFC 2396 and RFC 2732 with the JavaDoc's deviations in one JavaScript regular expression.
package java.net:

  @jvmClass("java/net/URLEncoder")
  object URLEncoder:
    private def keptAsIs(c: Char): Boolean =
      (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '.' || c == '-' || c == '*' || c == '_'

    private def wellFormedUntil(s: String, start: Int, end: Int): Int =
      var i = start
      var stop = false
      while !stop && i < end do
        val c = s.charAt(i)
        if Character.isHighSurrogate(c) && i + 1 < end && Character.isLowSurrogate(s.charAt(i + 1)) then i += 2
        else if Character.isSurrogate(c) then stop = true
        else i += 1
      i

    @deprecated("use encode(s, charset)", "")
    def encode(s: String): String = encode(s, java.nio.charset.StandardCharsets.UTF_8)

    def encode(s: String, enc: String): String =
      if !java.nio.charset.Charset.isSupported(enc) then throw new java.io.UnsupportedEncodingException(enc)
      encode(s, java.nio.charset.Charset.forName(enc))

    def encode(s: String, charset: java.nio.charset.Charset): String =
      val len = s.length
      val out = new java.lang.StringBuilder()
      var i = 0
      while i < len do
        val c = s.charAt(i)
        if keptAsIs(c) then
          out.append(c)
          i += 1
        else if c == ' ' then
          out.append('+')
          i += 1
        else
          val start = i
          while i < len && !keptAsIs(s.charAt(i)) && s.charAt(i) != ' ' do i += 1
          for b <- s.substring(start, wellFormedUntil(s, start, i)).getBytes(charset) do
            val v = b & 0xff
            out.append('%')
            out.append("0123456789ABCDEF".charAt(v >> 4))
            out.append("0123456789ABCDEF".charAt(v & 0xf))
      out.toString

  @jvmClass("java/net/URISyntaxException")
  class URISyntaxException(input: String, reason: String, index: Int) extends Exception(null):
    def this(input: String, reason: String) = this(input, reason, -1)
    def getInput(): String = input
    def getReason(): String = reason
    def getIndex(): Int = index
    override def getMessage(): String =
      reason + (if index >= 0 then " at index " + index else "") + ": " + input

  @jvmClass("java/net/URI")
  final class URI(origStr: String) extends java.io.Serializable with java.lang.Comparable[URI]:
    private val parts: Array[String] = URI.exec(URI.uriRe, origStr)
    if parts == null then throw new URISyntaxException(origStr, "Malformed URI")

    private val absolute = parts(URI.AbsScheme) != null
    private val opaque = parts(URI.AbsOpaquePart) != null
    private def part(abs: Int, rel: Int): String = if absolute then parts(abs) else parts(rel)

    private val scheme: String = parts(URI.AbsScheme)
    private val ssp: String =
      if !absolute then parts(URI.RelSchemeSpecificPart)
      else if opaque then parts(URI.AbsOpaquePart)
      else parts(URI.AbsHierPart)
    private val authority: String =
      val a = part(URI.AbsAuthority, URI.RelAuthority)
      if a == "" then null else a
    private val userInfo: String = part(URI.AbsUserInfo, URI.RelUserInfo)
    private val host: String = part(URI.AbsHost, URI.RelHost)
    private val port: Int =
      val p = part(URI.AbsPort, URI.RelPort)
      if p == null then -1 else Integer.parseInt(p)
    private val path: String =
      if part(URI.AbsAuthority, URI.RelAuthority) != null then
        val net = part(URI.AbsNetPath, URI.RelNetPath)
        if net == null then "" else net
      else if absolute then parts(URI.AbsAbsPath)
      else if parts(URI.RelAbsPath) != null then parts(URI.RelAbsPath)
      else parts(URI.RelRelPath)
    private val query: String = part(URI.AbsQuery, URI.RelQuery)
    private val fragment: String = parts(URI.Fragment)

    def this(scheme: String, ssp: String, fragment: String) = this(URI.uriStr(scheme, ssp, fragment))

    def this(scheme: String, userInfo: String, host: String, port: Int, path: String, query: String, fragment: String) =
      this(URI.uriStr(scheme, userInfo, host, port, path, query, fragment))
      parseServerAuthority()

    def this(scheme: String, host: String, path: String, fragment: String) =
      this(scheme, null, host, -1, path, null, fragment)

    // The JavaDoc asks for parseServerAuthority() here; the JDK does not call it, which keeps
    // registry-based authorities.
    def this(scheme: String, authority: String, path: String, query: String, fragment: String) =
      this(URI.uriStr(scheme, authority, path, query, fragment))

    def compareTo(that: URI): Int =
      def pathQueryFragment(): Int =
        val p = URI.escapeAwareCompare(path, that.path)
        if p != 0 then p
        else
          val q = URI.escapeAwareCompare(query, that.query)
          if q != 0 then q else URI.escapeAwareCompare(fragment, that.fragment)
      val s = URI.caseInsensitiveCompare(scheme, that.scheme)
      if s != 0 then s
      else if opaque != that.opaque then (if opaque then 1 else -1)
      else if opaque then
        val c = URI.escapeAwareCompare(ssp, that.ssp)
        if c != 0 then c else pathQueryFragment()
      else if host != null && that.host != null then
        val u = URI.escapeAwareCompare(userInfo, that.userInfo)
        if u != 0 then u
        else
          val h = URI.caseInsensitiveCompare(host, that.host)
          if h != 0 then h
          else if port != that.port then port - that.port
          else pathQueryFragment()
      else
        val a = URI.escapeAwareCompare(authority, that.authority)
        if a != 0 then a else pathQueryFragment()

    override def equals(that: Any): Boolean = that match
      case that: URI => compareTo(that) == 0
      case _ => false

    def getAuthority(): String = URI.decodeComponent(authority)
    def getFragment(): String = URI.decodeComponent(fragment)
    def getHost(): String = host
    def getPath(): String = URI.decodeComponent(path)
    def getPort(): Int = port
    def getQuery(): String = URI.decodeComponent(query)
    def getRawAuthority(): String = authority
    def getRawFragment(): String = fragment
    def getRawPath(): String = path
    def getRawQuery(): String = query
    def getRawSchemeSpecificPart(): String = ssp
    def getRawUserInfo(): String = userInfo
    def getScheme(): String = scheme
    def getSchemeSpecificPart(): String = URI.decodeComponent(ssp)
    def getUserInfo(): String = URI.decodeComponent(userInfo)

    override def hashCode(): Int =
      import scala.util.hashing.MurmurHash3.{mix, mixLast, finalizeHash}
      def escapes(s: String): Int = if s == null then 0 else URI.normalizeEscapes(s).hashCode
      var acc = URI.uriSeed
      acc = mix(acc, if scheme == null then 0 else scheme.toLowerCase.hashCode)
      if opaque then acc = mix(acc, escapes(ssp))
      else if host != null then
        acc = mix(acc, escapes(userInfo))
        acc = mix(acc, host.toLowerCase.hashCode)
        acc = mix(acc, port)
      else acc = mix(acc, escapes(authority))
      acc = mix(acc, escapes(path))
      acc = mix(acc, escapes(query))
      acc = mixLast(acc, escapes(fragment))
      finalizeHash(acc, 3)

    def isAbsolute(): Boolean = absolute
    def isOpaque(): Boolean = opaque

    def normalize(): URI =
      if opaque || path == null then this
      else
        val segments = URI.segments(path)
        val n = segments.length
        val start = if n != 0 && segments(0) == "" then 1 else 0
        var in = start
        var out = start
        while in != n do
          val segment = segments(in)
          in += 1
          if segment == "." then
            if in == n then
              segments(out) = ""
              out += 1
          else if segment == ".." then
            val droppable = out != start && segments(out - 1) != ".." && segments(out - 1) != ""
            if !droppable then
              segments(out) = ".."
              out += 1
            else if in == n then segments(out - 1) = ""
            else out -= 1
          else if segment != "" || in == n then
            segments(out) = segment
            out += 1
        val kept = segments.take(out)
        val newPath = (if out != 0 && kept(0).contains(":") then "./" else "") + kept.mkString("/")
        if newPath == path then this
        else new URI(getScheme(), getRawAuthority(), newPath, getQuery(), getFragment())

    def parseServerAuthority(): URI =
      if authority != null && host == null then throw new URISyntaxException(origStr, "No Host in URI")
      this

    def relativize(uri: URI): URI =
      if opaque || uri.opaque || scheme != uri.scheme || URI.escapeAwareCompare(authority, uri.authority) != 0 then uri
      else
        val base = normalize().getRawPath()
        val target = uri.normalize().getRawPath()
        if target.startsWith(base) then
          val rest = target.substring(base.length)
          new URI(null, null, if rest.startsWith("/") then rest.substring(1) else rest, uri.getQuery(), uri.getFragment())
        else uri

    def resolve(str: String): URI = resolve(URI.create(str))

    def resolve(uri: URI): URI =
      if uri.absolute || opaque then uri
      else if uri.scheme == null && uri.authority == null && uri.path == "" && uri.query == null then
        new URI(getScheme(), getRawAuthority(), getRawPath(), getRawQuery(), uri.getRawFragment())
      else if uri.authority != null then
        new URI(getScheme(), uri.getRawAuthority(), uri.getRawPath(), uri.getRawQuery(), uri.getRawFragment())
      else if uri.path.startsWith("/") then
        new URI(getScheme(), getRawAuthority(), uri.getRawPath(), uri.getRawQuery(), uri.getRawFragment())
      else
        val end = path.lastIndexOf('/')
        val joined = if end == -1 then uri.path else path.substring(0, end + 1) + uri.path
        new URI(getScheme(), getAuthority(), joined, uri.getRawQuery(), uri.getRawFragment()).normalize()

    def toASCIIString(): String = URI.quote(origStr, URI.nonAsciiRe)

    override def toString(): String = origStr

  @jvmClass("java/net/URI")
  object URI:
    def create(str: String): URI =
      try new URI(str)
      catch case e: URISyntaxException => throw new IllegalArgumentException(e.getMessage(), e)

    private[net] final val AbsScheme = 1
    private[net] final val AbsHierPart = 2
    private[net] final val AbsAuthority = 3
    private[net] final val AbsUserInfo = 4
    private[net] final val AbsHost = 5
    private[net] final val AbsPort = 6
    private[net] final val AbsNetPath = 7
    private[net] final val AbsAbsPath = 8
    private[net] final val AbsQuery = 9
    private[net] final val AbsOpaquePart = 10
    private[net] final val RelSchemeSpecificPart = 11
    private[net] final val RelAuthority = 12
    private[net] final val RelUserInfo = 13
    private[net] final val RelHost = 14
    private[net] final val RelPort = 15
    private[net] final val RelNetPath = 16
    private[net] final val RelAbsPath = 17
    private[net] final val RelRelPath = 18
    private[net] final val RelQuery = 19
    private[net] final val Fragment = 20
    private[net] final val uriSeed = 53722356

    private val ipv4address =
      val digit = "(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)"
      "(?:" + digit + "\\.){3}" + digit

    private val ipv6address =
      val block = "[0-9a-f]{1,4}"
      val l = "(?:" + block + ":)"
      val r = "(?::" + block + ")"
      val v4 = ipv4address
      "(?:" + l + "{7}" + block + "|" + l + "{1,7}:|" + l + "{1,6}" + r + "|" + l + "{1,5}" + r + "{1,2}|" +
        l + "{1,4}" + r + "{1,3}|" + l + "{1,3}" + r + "{1,4}|" + l + "{1,2}" + r + "{1,5}|" + l + r + "{1,6}|" +
        ":(?:" + r + "{1,7}|:)|" + l + "{6}" + v4 + "|" + l + "{1,5}:" + v4 + "|" + l + "{1,4}" + r + ":" + v4 + "|" +
        l + "{1,3}" + r + "{1,2}:" + v4 + "|" + l + "{1,2}" + r + "{1,3}:" + v4 + "|" + l + r + "{1,4}:" + v4 + "|" +
        "::" + l + "{1,5}" + v4 + ")(?:%[0-9a-z]+)?"

    private val ipv6Re: Any = regExp("^" + ipv6address + "$", "i")

    private[net] val uriRe: Any =
      val escaped = "%[a-f0-9]{2}"
      val other = "[^\\u0000-\\u00a0\\u1680\\u2000-\\u200a\\u202f\\u205f\\u3000\\u2028\\u2029]"
      val uric = "(?:[;/?:@&=+$,\\[\\]a-z0-9-_.!~*'()]|" + escaped + "|" + other + ")"
      val pchar = "(?:[a-z0-9-_.!~*'():@&=+$,]|" + escaped + "|" + other + ")"
      val domainlabel = "(?:[a-z0-9]|[a-z0-9][a-z0-9-]*[a-z0-9])"
      val toplabel = "(?:[a-z]|[a-z][a-z0-9-]*[a-z0-9])"
      val hostname = "(?:" + domainlabel + "\\.)*" + toplabel + "\\.?"
      val host = "(" + hostname + "|" + ipv4address + "|\\[(?:" + ipv6address + ")\\])"
      val hostport = host + "(?::([0-9]*))?"
      val userinfo = "(?:[a-z0-9-_.!~*'();:&=+$,]|" + escaped + "|" + other + ")*"
      val server = "(?:(?:(" + userinfo + ")@)?" + hostport + ")?"
      val regName = "(?:[a-z0-9-_.!~*'()$,;:@&=+]|" + escaped + "|" + other + ")+"
      val authority = server + "|" + regName
      val segment = pchar + "*(?:;" + pchar + "*)*"
      val absPath = "/" + segment + "(?:/" + segment + ")*"
      val netPath = "//(" + authority + ")(" + absPath + ")?"
      val relPath = "(?:[a-z0-9-_.!~*'();@&=+$,]|" + escaped + ")*(?:" + absPath + ")?"
      val query = "(" + uric + "*)"
      val fragment = "(" + uric + "*)"
      val hierPart = "(?:" + netPath + "|(" + absPath + "))(?:\\?" + query + ")?"
      val opaquePart = "(?:[a-z0-9-_.!~*'();?:@&=+$,]|" + escaped + ")" + uric + "*"
      val scheme = "([a-z][a-z0-9+-.]*)"
      val absoluteURI = scheme + ":(?:(" + hierPart + ")|(" + opaquePart + "))"
      val relativeURI = "((?:" + netPath + "|(" + absPath + ")|(" + relPath + "))(?:\\?" + query + ")?)"
      regExp("^(?:" + absoluteURI + "|" + relativeURI + ")(?:#" + fragment + ")?$", "i")

    // Each matches a character the component cannot hold unquoted, or a `%` that starts no escape.
    private val notOther = "\\u007f-\\u00a0\\u1680\\u2000-\\u200a\\u202f\\u205f\\u3000\\u2028\\u2029]|%(?![0-9a-f]{2})"
    private val userInfoRe: Any = regExp("[\\u0000- \"#/<>?@\\[-\\^`{-}" + notOther, "ig")
    private val pathRe: Any = regExp("[\\u0000- \"#<>?\\[-\\^`{-}" + notOther, "ig")
    private val authorityRe: Any = regExp("[\\u0000- \"#/<>?\\^`{-}" + notOther, "ig")
    private val illegalRe: Any = regExp("[\\u0000- \"#<>@\\^`{-}" + notOther, "ig")
    private[net] val nonAsciiRe: Any = regExp("[^\\u0000-\\u007F]+", "g")

    // The bodies are the interpreter's, over java.util.regex's syntax, which reads these
    // expressions as JavaScript does; `g` is a flag of the replacements alone.
    @js("new RegExp($1, $2)")
    private def regExp(source: String, flags: String): Any =
      new scala.util.matching.Regex((if flags.indexOf('i') >= 0 then "(?i)" else "") + source)

    @js("(() => { const m = $1.exec($2); return m === null ? null : Array.from(m, (x) => x === undefined ? null : x); })()")
    private[net] def exec(re: Any, s: String): Array[String] =
      re.asInstanceOf[scala.util.matching.Regex].findFirstMatchIn(s) match
        case Some(m) => Array.tabulate(m.groupCount + 1)(i => m.group(i))
        case None => null

    @js("$1.split(\"/\")")
    private[net] def segments(path: String): Array[String] = path.split("/", -1)

    @js("$1.replace($2, (s) => Array.from(new TextEncoder().encode(s), (b) => (b < 16 ? \"%0\" : \"%\") + b.toString(16).toUpperCase()).join(\"\"))")
    private[net] def quote(s: String, re: Any): String =
      re.asInstanceOf[scala.util.matching.Regex].replaceAllIn(s, m =>
        val escaped = new java.lang.StringBuilder()
        for b <- m.matched.getBytes("UTF-8") do
          val v = b & 0xff
          escaped.append(if v < 16 then "%0" else "%").append(Integer.toHexString(v).toUpperCase)
        scala.util.matching.Regex.quoteReplacement(escaped.toString))

    @js("$1 === null || $1.indexOf(\"%\") < 0 ? $1 : $1.replace(/(?:%[0-9a-fA-F]{2})+/g, (s) => new TextDecoder().decode(Uint8Array.from(s.slice(1).split(\"%\"), (h) => parseInt(h, 16))))")
    private[net] def decodeComponent(s: String): String =
      if s == null || s.indexOf('%') < 0 then s
      else
        new scala.util.matching.Regex("(?:%[0-9a-fA-F]{2})+").replaceAllIn(s, m =>
          val bytes = m.matched.substring(1).split("%").map(h => Integer.parseInt(h, 16).toByte)
          scala.util.matching.Regex.quoteReplacement(new String(bytes, "UTF-8")))

    private[net] def uriStr(scheme: String, ssp: String, fragment: String): String =
      var s = ""
      if scheme != null then s += scheme + ":"
      if ssp != null then s += quote(ssp, illegalRe)
      if fragment != null then s += "#" + quote(fragment, illegalRe)
      s

    private[net] def uriStr(scheme: String, userInfo: String, host: String, port: Int, path: String, query: String, fragment: String): String =
      var s = ""
      if scheme != null then s += scheme + ":"
      if userInfo != null || host != null || port != -1 then s += "//"
      if userInfo != null then s += quote(userInfo, userInfoRe) + "@"
      if host != null then s += (if matches(ipv6Re, host) then "[" + host + "]" else host)
      if port != -1 then s += ":" + port
      if path != null then s += quote(path, pathRe)
      if query != null then s += "?" + quote(query, illegalRe)
      if fragment != null then s += "#" + quote(fragment, illegalRe)
      s

    private[net] def uriStr(scheme: String, authority: String, path: String, query: String, fragment: String): String =
      var s = ""
      if scheme != null then s += scheme + ":"
      if authority != null then s += "//" + quote(authority, authorityRe)
      if path != null then s += quote(path, pathRe)
      if query != null then s += "?" + quote(query, illegalRe)
      if fragment != null then s += "#" + quote(fragment, illegalRe)
      s

    @js("$1.test($2)")
    private def matches(re: Any, s: String): Boolean = re.asInstanceOf[scala.util.matching.Regex].findFirstIn(s).isDefined

    /** `null` is smaller than any other value. */
    private[net] def caseInsensitiveCompare(x: String, y: String): Int =
      if x == null then (if y == null then 0 else -1)
      else if y == null then 1
      else x.compareToIgnoreCase(y)

    /** Case-sensitive except inside escapes: `a%A0` and `a%a0` are equal, `a%A0` and `A%A0` not. */
    private[net] def escapeAwareCompare(x: String, y: String): Int =
      if x == null then (if y == null then 0 else -1)
      else if y == null then 1
      else
        var i = 0
        var result = 0
        var done = false
        while !done do
          if i >= x.length || i >= y.length then
            result = x.length - y.length
            done = true
          else
            val diff = x.charAt(i) - y.charAt(i)
            if diff != 0 then
              result = diff
              done = true
            else if x.charAt(i) == '%' then
              val c = x.substring(i + 1, i + 3).compareToIgnoreCase(y.substring(i + 1, i + 3))
              if c != 0 then
                result = c
                done = true
              else i += 3
            else i += 1
        result

    private[net] def normalizeEscapes(s: String): String =
      var i = 0
      var out = ""
      while i < s.length do
        if s.charAt(i) == '%' then
          out += s.substring(i, i + 3).toUpperCase
          i += 3
        else
          out += s.substring(i, i + 1)
          i += 1
      out
