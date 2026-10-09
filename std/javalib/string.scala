// The methods of `java.lang.String` as intrinsics of the builtin `String`, which is the
// JavaScript string: the members library bodies and programs call on it, in the JDK's shapes.
// They stay under `--std=scala-library`, where `StringOps` comes from the jar and calls them.
package scala:

  extension (s: String)
    @javaDefined @js("$0.length")
    @jvm("invokevirtual java/lang/String.length()I")
    def length: Int
    @js("$charAt($0, $1)")
    @jvm("invokevirtual java/lang/String.charAt(I)C")
    def charAt(i: Int): Char
    @js("$codePointAt($0, $1)")
    @jvm("invokevirtual java/lang/String.codePointAt(I)I")
    def codePointAt(i: Int): Int
    @js("$substring($0, $1, $2)")
    @jvm("invokevirtual java/lang/String.substring(II)Ljava/lang/String;")
    def substring(start: Int, end: Int): String
    @js("$substring($0, $1)")
    @jvm("invokevirtual java/lang/String.substring(I)Ljava/lang/String;")
    def substring(start: Int): String
    @js("$0.indexOf($1)")
    @jvm("$0 $1:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String; invokevirtual java/lang/String.indexOf(Ljava/lang/String;)I")
    def indexOf(part: String | Char): Int
    @js("$0.indexOf($fromCodePoint($1))")
    @jvm("invokevirtual java/lang/String.indexOf(I)I")
    def indexOf(ch: Int): Int
    @js("$0.indexOf($fromCodePoint($1), $2)")
    @jvm("invokevirtual java/lang/String.indexOf(II)I")
    def indexOf(ch: Int, from: Int): Int
    @js("$0.lastIndexOf($fromCodePoint($1))")
    @jvm("invokevirtual java/lang/String.lastIndexOf(I)I")
    def lastIndexOf(ch: Int): Int
    @js("$0.indexOf($1, $2)")
    @jvm("$0 $1:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String; $2:I invokevirtual java/lang/String.indexOf(Ljava/lang/String;I)I")
    def indexOf(part: String | Char, from: Int): Int
    @js("$0.lastIndexOf($1)")
    @jvm("$0 $1:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String; invokevirtual java/lang/String.lastIndexOf(Ljava/lang/String;)I")
    def lastIndexOf(part: String | Char): Int
    @js("($2 < 0 ? -1 : $0.lastIndexOf($fromCodePoint($1), $2))")
    @jvm("invokevirtual java/lang/String.lastIndexOf(II)I")
    def lastIndexOf(ch: Int, from: Int): Int
    @js("($2 < 0 ? -1 : $0.lastIndexOf($1, $2))")
    @jvm("$0 $1:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String; $2:I invokevirtual java/lang/String.lastIndexOf(Ljava/lang/String;I)I")
    def lastIndexOf(part: String | Char, from: Int): Int
    @js("$0.includes($1)")
    @jvm("$0 $1:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String; invokevirtual java/lang/String.contains(Ljava/lang/CharSequence;)Z")
    def contains(part: String | Char): Boolean
    @js("$0.startsWith($1)")
    @jvm("invokevirtual java/lang/String.startsWith(Ljava/lang/String;)Z")
    def startsWith(prefix: String): Boolean
    @jvm("invokevirtual java/lang/String.startsWith(Ljava/lang/String;I)Z")
    def startsWith(prefix: String, offset: Int): Boolean =
      offset >= 0 && offset <= s.length - prefix.length && s.substring(offset, offset + prefix.length) == prefix
    @js("$0.endsWith($1)")
    @jvm("invokevirtual java/lang/String.endsWith(Ljava/lang/String;)Z")
    def endsWith(suffix: String): Boolean
    // Java's toUpperCase(), toUpperCase(locale) and trim(), next to the forms without parentheses.
    def toUpperCase(locale: Any = ()): String = s.toUpperCase
    def toLowerCase(locale: Any = ()): String = s.toLowerCase
    def trim(parens: Unit = ()): String = s.trim
    @js("$0.toUpperCase()")
    @jvm("invokevirtual java/lang/String.toUpperCase()Ljava/lang/String;")
    def toUpperCase: String
    @js("$0.toLowerCase()")
    @jvm("invokevirtual java/lang/String.toLowerCase()Ljava/lang/String;")
    def toLowerCase: String
    @js("$trim($0)")
    @jvm("invokevirtual java/lang/String.trim()Ljava/lang/String;")
    def trim: String
    @js("$strip($0, 0, 1)")
    @jvm("invokevirtual java/lang/String.stripTrailing()Ljava/lang/String;")
    def stripTrailing: String
    @js("$strip($0, 1, 0)")
    @jvm("invokevirtual java/lang/String.stripLeading()Ljava/lang/String;")
    def stripLeading: String
    @javaDefined @js("($0.length === 0)")
    @jvm("invokevirtual java/lang/String.isEmpty()Z")
    def isEmpty: Boolean
    @js("$split($0, $1)")
    def split(separator: String | Char): Array[String] = scala.runtime.splitString(s, scala.runtime.separatorRegex(separator), 0)
    @js("$split($0, $1, $2)")
    def split(separator: String | Char, limit: Int): Array[String] = scala.runtime.splitString(s, scala.runtime.separatorRegex(separator), limit)
    @js("$reReplace($1, $0, $2, true)")
    @jvm("invokevirtual java/lang/String.replaceAll(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;")
    def replaceAll(regex: String, replacement: String): String
    @js("$reReplace($1, $0, $2, false)")
    @jvm("invokevirtual java/lang/String.replaceFirst(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;")
    def replaceFirst(regex: String, replacement: String): String
    @js("($reExec($1, \"f\", $0) !== null)")
    @jvm("invokevirtual java/lang/String.matches(Ljava/lang/String;)Z")
    def matches(regex: String): Boolean
    @js("$equalsIgnoreCase($0, $1)")
    @jvm("invokevirtual java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z")
    def equalsIgnoreCase(that: String): Boolean
    @js("$compareStrings($0.toLowerCase(), $1.toLowerCase())")
    @jvm("invokevirtual java/lang/String.compareToIgnoreCase(Ljava/lang/String;)I")
    def compareToIgnoreCase(that: String): Int
    @js("$regionMatches($0, $1, $2, $3, $4, $5)")
    @jvm("invokevirtual java/lang/String.regionMatches(ZILjava/lang/String;II)Z")
    def regionMatches(ignoreCase: Boolean, toffset: Int, other: String, ooffset: Int, len: Int): Boolean
    @jvm("invokevirtual java/lang/String.regionMatches(ILjava/lang/String;II)Z")
    def regionMatches(toffset: Int, other: String, ooffset: Int, len: Int): Boolean =
      if toffset < 0 || ooffset < 0 || toffset > s.length - len || ooffset > other.length - len then false
      else
        var i = 0
        while i < len && s.charAt(toffset + i) == other.charAt(ooffset + i) do i += 1
        i >= len
    @js("$repeat($0, $1)")
    @jvm("invokevirtual java/lang/String.repeat(I)Ljava/lang/String;")
    def repeat(n: Int): String
    @js("($0 + $1)")
    @jvm("invokevirtual java/lang/String.concat(Ljava/lang/String;)Ljava/lang/String;")
    def concat(other: String): String
    @javaDefined @js("$strip($0, 1, 1)")
    @jvm("invokevirtual java/lang/String.strip()Ljava/lang/String;")
    def strip: String
    @javaDefined @js("$0")
    @jvm("invokevirtual java/lang/String.intern()Ljava/lang/String;")
    def intern: String
    @javaDefined @js("($strip($0, 1, 0).length === 0)")
    @jvm("invokevirtual java/lang/String.isBlank()Z")
    def isBlank: Boolean
    @js("$getChars($0, $1, $2, $3, $4)")
    @jvm("invokevirtual java/lang/String.getChars(II[CI)V")
    def getChars(srcBegin: Int, srcEnd: Int, dst: Array[Char], dstBegin: Int): Unit
    @javaDefined @js("$0.split(\"\")")
    @jvm("invokevirtual java/lang/String.toCharArray()[C")
    def toCharArray: Array[Char] =
      val out = Array.empty[Char]
      var i = 0
      while i < s.length do
        out.push(s.charAt(i))
        i += 1
      out
    @js("$replace($0, $1, $2)")
    @jvm("$0 $1:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String; $2:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String; invokevirtual java/lang/String.replace(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Ljava/lang/String;")
    def replace(target: String | Char, replacement: String | Char): String
    @js("$compareStrings($0, $1)")
    @jvm("invokevirtual java/lang/String.compareTo(Ljava/lang/String;)I")
    def compareTo(that: String): Int

package java.lang:
  // The statics of `java.lang.String`; what is missing here is read from the JDK's class file.
  @javaDefined
  @jvmClass("java/lang/String")
  object String:
    @js("$str($1)")
    @jvm("invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String;")
    def valueOf(x: Any): String
    @js("$1")
    @jvm("invokestatic java/lang/String.valueOf(C)Ljava/lang/String;")
    def valueOf(c: Char): String
    @js("$1.join(\"\")")
    @jvm("invokestatic java/lang/String.valueOf([C)Ljava/lang/String;")
    def valueOf(chars: Array[Char]): String
    // `new String(...)`: the typer sends the constructors of the builtin string here.
    @js("$1.slice($2, $2 + $3).join(\"\")")
    @jvm("new java/lang/String dup $1 $2:I $3:I invokespecial java/lang/String.<init>([CII)V")
    def newString(chars: Array[Char], offset: Int, count: Int): String
    @js("$1.join(\"\")")
    @jvm("invokestatic java/lang/String.valueOf([C)Ljava/lang/String;")
    def newString(chars: Array[Char]): String
    @js("$1")
    def newString(s: String): String
    @js("\"\"")
    @jvm("ldc \"\"")
    def newString(): String
    @jvm("new java/lang/String dup $1 invokespecial java/lang/String.<init>(Ljava/lang/StringBuffer;)V")
    def newString(buffer: java.lang.StringBuffer): String = buffer.toString
    @jvm("new java/lang/String dup $1 invokespecial java/lang/String.<init>(Ljava/lang/StringBuilder;)V")
    def newString(builder: java.lang.StringBuilder): String = builder.toString
    @jvm("new java/lang/String dup rt $1:L rtcall javaBytes(Ljava/lang/Object;)Ljava/lang/Object; checkcast [B invokespecial java/lang/String.<init>([B)V")
    def newString(bytes: Array[scala.Byte]): String = java.nio.charset.decode(bytes, 0, bytes.length, java.nio.charset.StandardCharsets.UTF_8)
    @jvm("new java/lang/String dup rt $1:L rtcall javaBytes(Ljava/lang/Object;)Ljava/lang/Object; checkcast [B $2 invokespecial java/lang/String.<init>([BLjava/lang/String;)V")
    def newString(bytes: Array[scala.Byte], charsetName: String): String = java.nio.charset.decode(bytes, 0, bytes.length, java.nio.charset.Charset.forName(charsetName))
    @jvm("new java/lang/String dup rt $1:L rtcall javaBytes(Ljava/lang/Object;)Ljava/lang/Object; checkcast [B $2 invokespecial java/lang/String.<init>([BLjava/nio/charset/Charset;)V")
    def newString(bytes: Array[scala.Byte], charset: java.nio.charset.Charset): String = java.nio.charset.decode(bytes, 0, bytes.length, charset)
    @jvm("new java/lang/String dup rt $1:L rtcall javaBytes(Ljava/lang/Object;)Ljava/lang/Object; checkcast [B $2:I $3:I $4 invokespecial java/lang/String.<init>([BIILjava/lang/String;)V")
    def newString(bytes: Array[scala.Byte], offset: Int, length: Int, charsetName: String): String = java.nio.charset.decode(bytes, offset, length, java.nio.charset.Charset.forName(charsetName))
    @jvm("new java/lang/String dup rt $1:L rtcall javaBytes(Ljava/lang/Object;)Ljava/lang/Object; checkcast [B $2:I $3:I $4 invokespecial java/lang/String.<init>([BIILjava/nio/charset/Charset;)V")
    def newString(bytes: Array[scala.Byte], offset: Int, length: Int, charset: java.nio.charset.Charset): String = java.nio.charset.decode(bytes, offset, length, charset)
    @js("$format($1, [...$2])")
    def format(format: String, args: Any*): String
