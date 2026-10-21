// Adapted from scalafix's DisableSyntax and DisableSyntaxConfig
// (https://github.com/scalacenter/scalafix, commit 41cf7983a19a0453be78d1fe9863d77470ca6dec,
// scalafix-rules/src/main/scala/scalafix/internal/rule/DisableSyntax.scala and DisableSyntaxConfig.scala).
//
// BSD 3-Clause License
//
// Copyright (c) 2016-2023, EPFL
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//
// The port's local changes: the configuration is decoded by hand where metaconfig derives it; scalameta's
// `paramClauseGroups` are the definition's parameter clauses, a parameter's `Mod.Implicit` its clause's `implicit`;
// `bfsCollectEach` is a pre-order `collect`, the lints' order being the printer's. `noXml` is decoded and reports
// nothing: teq reads no XML literal (docs/FIX.md).
package teq.fix.rules

import teq.fix.*

final case class DisableSyntaxRegex(pattern: String, captureGroup: Int, message: Option[String], id: Option[String])

final case class DisableSyntaxConfig(
    keywords: Set[String] = Set(),
    noVars: Boolean = false,
    noThrows: Boolean = false,
    noNulls: Boolean = false,
    noReturns: Boolean = false,
    noWhileLoops: Boolean = false,
    noAsInstanceOf: Boolean = false,
    noIsInstanceOf: Boolean = false,
    noSemicolons: Boolean = false,
    noTabs: Boolean = false,
    noXml: Boolean = false,
    noCovariantTypes: Boolean = false,
    noContravariantTypes: Boolean = false,
    noDefaultArgs: Boolean = false,
    noValInAbstract: Boolean = false,
    noImplicitObject: Boolean = false,
    noImplicitConversion: Boolean = false,
    noFinalVal: Boolean = false,
    noFinalize: Boolean = false,
    noValPatterns: Boolean = false,
    noUniversalEquality: Boolean = false,
    noUniversalEqualityMessage: String = "== and != are unsafe since they allow comparing two unrelated types",
    regex: List[DisableSyntaxRegex] = Nil
):
  def isDisabled(keyword: String): Boolean = keyword match
    case "var" if noVars => true
    case "throw" if noThrows => true
    case "null" if noNulls => true
    case "return" if noReturns => true
    case "while" if noWhileLoops => true
    case _ => keywords.contains(keyword)

object DisableSyntaxConfig:
  val default: DisableSyntaxConfig = DisableSyntaxConfig()

  val keywordsAll: List[String] = List(
    "abstract", "case", "catch", "class", "def", "do", "else", "enum", "extends", "false", "final", "finally", "for",
    "forSome", "if", "implicit", "import", "lazy", "match", "macro", "new", "null", "object", "override", "package",
    "private", "protected", "return", "sealed", "super", "this", "throw", "trait", "true", "try", "type", "val",
    "var", "while", "with", "yield"
  )

  def decode(c: Conf): Configured[DisableSyntaxConfig] =
    val d = default
    for
      keywords <- c.get("keywords").fold(Configured.Ok(Set.empty[String]))(k => k.asList.flatMap(l => Configured.all(l.map(keyword))).map(_.toSet))
      noVars <- c.boolean("noVars", d.noVars)
      noThrows <- c.boolean("noThrows", d.noThrows)
      noNulls <- c.boolean("noNulls", d.noNulls)
      noReturns <- c.boolean("noReturns", d.noReturns)
      noWhileLoops <- c.boolean("noWhileLoops", d.noWhileLoops)
      noAsInstanceOf <- c.boolean("noAsInstanceOf", d.noAsInstanceOf)
      noIsInstanceOf <- c.boolean("noIsInstanceOf", d.noIsInstanceOf)
      noSemicolons <- c.boolean("noSemicolons", d.noSemicolons)
      noTabs <- c.boolean("noTabs", d.noTabs)
      noXml <- c.boolean("noXml", d.noXml)
      noCovariantTypes <- c.boolean("noCovariantTypes", d.noCovariantTypes)
      noContravariantTypes <- c.boolean("noContravariantTypes", d.noContravariantTypes)
      noDefaultArgs <- c.boolean("noDefaultArgs", d.noDefaultArgs)
      noValInAbstract <- c.boolean("noValInAbstract", d.noValInAbstract)
      noImplicitObject <- c.boolean("noImplicitObject", d.noImplicitObject)
      noImplicitConversion <- c.boolean("noImplicitConversion", d.noImplicitConversion)
      noFinalVal <- c.boolean("noFinalVal", d.noFinalVal)
      noFinalize <- c.boolean("noFinalize", d.noFinalize)
      noValPatterns <- c.boolean("noValPatterns", d.noValPatterns)
      noUniversalEquality <- c.boolean("noUniversalEquality", d.noUniversalEquality)
      noUniversalEqualityMessage <- c.string("noUniversalEqualityMessage", d.noUniversalEqualityMessage)
      regex <- c.get("regex").fold(Configured.Ok(Nil))(r => r.asList.flatMap(l => Configured.all(l.map(regexEntry))))
    yield DisableSyntaxConfig(keywords, noVars, noThrows, noNulls, noReturns, noWhileLoops, noAsInstanceOf, noIsInstanceOf,
      noSemicolons, noTabs, noXml, noCovariantTypes, noContravariantTypes, noDefaultArgs, noValInAbstract, noImplicitObject,
      noImplicitConversion, noFinalVal, noFinalize, noValPatterns, noUniversalEquality, noUniversalEqualityMessage, regex)

  // `DisabledKeyword`'s decoder: a keyword, `null`, `true` or `false`.
  private def keyword(c: Conf): Configured[String] =
    val word = c.kind match
      case "null" => Configured.Ok("null")
      case "boolean" | "string" => c.asString
      case k => Configured.NotOk(c.place + ": Type mismatch;\n  found    : " + k + "\n  expected : String")
    word.flatMap { k =>
      if keywordsAll.contains(k) then Configured.Ok(k)
      else
        val closest = keywordsAll.minBy(levenshtein(k))
        val didYouMean = if levenshtein(k)(closest) / k.length.toDouble < 0.20 then " (Did you mean: " + closest + "?)" else ""
        Configured.NotOk(c.place + ": " + k + " is not in our supported keywords." + didYouMean)
    }

  // `CustomMessage[Either[Regex, Pattern]]`: a pattern as a string, or an object with `pattern` or with
  // `regex = { pattern, captureGroup }`, each with an optional `message` and `id`; the pattern read MULTILINE.
  private def regexEntry(c: Conf): Configured[DisableSyntaxRegex] =
    def compiled(p: Conf): Configured[String] = p.asString.flatMap { s =>
      try
        java.util.regex.Pattern.compile(s, java.util.regex.Pattern.MULTILINE)
        Configured.Ok(s)
      catch case e: java.util.regex.PatternSyntaxException => Configured.NotOk(p.place + ": " + e.getMessage)
    }
    def message(o: Conf): Configured[Option[String]] = o.get("message") match
      case None => Configured.Ok(None)
      case Some(m) => m.asString.map(msg => Some(if msg.contains("\n") then "\n" + msg.stripMargin else msg))
    def id(o: Conf): Configured[Option[String]] = o.get("id").fold(Configured.Ok(None))(_.asString.map(Some(_)))
    c.kind match
      case "object" =>
        (c.get("pattern"), c.get("regex")) match
          case (Some(p), _) =>
            for pattern <- compiled(p); m <- message(c); i <- id(c) yield DisableSyntaxRegex(pattern, 0, m, i)
          case (None, Some(r)) =>
            for
              pattern <- r.get("pattern").fold(Configured.NotOk(r.place + ": the regex has no pattern"))(compiled)
              group <- r.int("captureGroup", 0)
              m <- message(c)
              i <- id(c)
            yield DisableSyntaxRegex(pattern, group, m, i)
          case _ => Configured.NotOk(c.place + ": Failed to decode configuration for either of the following types: a pattern or a regex")
      case _ => compiled(c).map(p => DisableSyntaxRegex(p, 0, None, None))

  private def levenshtein(s1: String)(s2: String): Int =
    val dist = Array.tabulate(s2.length + 1, s1.length + 1) { (j, i) => if j == 0 then i else if i == 0 then j else 0 }
    for j <- 1 to s2.length; i <- 1 to s1.length do
      dist(j)(i) =
        if s2(j - 1) == s1(i - 1) then dist(j - 1)(i - 1)
        else dist(j - 1)(i).min(dist(j)(i - 1)).min(dist(j - 1)(i - 1)) + 1
    dist(s2.length)(s1.length)

final class DisableSyntax(config: DisableSyntaxConfig) extends SyntacticRule("DisableSyntax"):
  def this() = this(DisableSyntaxConfig())

  override def description: String = "Reports an error for disabled features such as var or XML literals."

  override def withConfiguration(c: Configuration): Configured[Rule] =
    val section = c.conf.get("disableSyntax").getOrElse(c.conf.section("DisableSyntax"))
    DisableSyntaxConfig.decode(section).map(new DisableSyntax(_))

  private def checkRegex(doc: SyntacticDocument): List[Diagnostic] =
    // Matches starting inside suppression anchors are never reported: an anchor may name the very pattern it
    // suppresses (scalafix issue 507).
    lazy val anchors: List[(Int, Int)] = doc.tokens.collect {
      case t if t.is("Comment") && Suppression.isAnchor(t.value) => (t.start, t.end)
    }
    def withinAnchor(position: Position): Boolean = anchors.exists { case (s, e) => s <= position.start && position.start < e }
    val text = doc.input.text
    config.regex.flatMap { r =>
      val matcher = java.util.regex.Pattern.compile(r.pattern, java.util.regex.Pattern.MULTILINE).matcher(text)
      val id = r.id.getOrElse(r.pattern)
      val message = r.message.getOrElse(r.pattern + " is disabled")
      val out = scala.collection.mutable.ListBuffer.empty[Diagnostic]
      while matcher.find() do
        val position =
          if matcher.group(r.captureGroup) == null then Position.Range(doc.input, matcher.start, matcher.end)
          else Position.Range(doc.input, matcher.start(r.captureGroup), matcher.end(r.captureGroup))
        if !withinAnchor(position) then
          val substituted = (0 to matcher.groupCount).foldLeft(message) { (msg, idx) =>
            val g = matcher.group(idx)
            if g != null then msg.replace("{$" + idx + "}", g) else msg
          }
          out += Diagnostic(id, substituted, position)
      out.toList
    }

  private def checkTokens(doc: SyntacticDocument): List[Diagnostic] =
    val tokens = doc.tokens.toList
    tokens.zipWithIndex.collect {
      case (token, _) if token.isKeyword && DisableSyntaxConfig.keywordsAll.contains(token.text) && config.isDisabled(token.text) =>
        val keyword = token.text
        keyword match
          case "var" => Diagnostic(keyword, "mutable state should be avoided", token.pos)
          case "null" => Diagnostic(keyword, "null should be avoided, consider using Option instead", token.pos)
          case "throw" => Diagnostic(keyword, "exceptions should be avoided, consider encoding the error in the return type instead", token.pos)
          case "return" => Diagnostic(keyword, "return should be avoided, consider using if/else instead", token.pos)
          case "while" => Diagnostic(keyword, "while loops should be avoided, consider using recursion instead", token.pos)
          case _ => Diagnostic(keyword, keyword + " is disabled", token.pos)
      case (token, _) if token.is("Semicolon") && config.noSemicolons =>
        Diagnostic("noSemicolons", "semicolons are disabled", token.pos)
      case (token, _) if token.is("Tab") && config.noTabs =>
        Diagnostic("noTabs", "tabs are disabled", token.pos)
      case (token, i) if token.is("Ident") && token.value == "asInstanceOf" && config.noAsInstanceOf =>
        val isMatchableCast = tokens.drop(i + 1).takeWhile(t => !t.is("RightBracket")).exists(t => t.is("Ident") && t.value == "Matchable")
        if isMatchableCast then
          Diagnostic(
            "asInstanceOfMatchable",
            "asInstanceOf[Matchable] is used here to enable pattern matching on Any. " +
              "Consider using the .asMatchable extension method instead for better readability.",
            token.pos
          )
        else Diagnostic("asInstanceOf", "asInstanceOf casts are disabled, use pattern matching instead", token.pos)
      case (token, _) if token.is("Ident") && token.value == "isInstanceOf" && config.noIsInstanceOf =>
        Diagnostic("isInstanceOf", "isInstanceOf checks are disabled, use pattern matching instead", token.pos)
    }

  private def abstractWithVals(t: Tree): Option[List[Defn.Val]] =
    val stats = t match
      case c: Defn.Class if c.mods.exists(_.isInstanceOf[Mod.Abstract]) => c.templ.stats
      case c: Defn.Trait => c.templ.stats
      case _ => Nil
    val vals = stats.collect { case v: Defn.Val => v }
    if vals.isEmpty then None else Some(vals)

  private def defaultArgs(t: Tree): Option[List[Term]] = t match
    case d: Defn.Def => Some(d.paramClauses.flatMap(_.values).flatMap(_.default))
    case _ => None

  private def isProhibited(p: Pat): Boolean = p match
    case _: Pat.Tuple | _: Pat.Var | _: Pat.Wildcard => false
    case _ => true

  private def noValPattern(t: Tree): Option[Tree] = t match
    case v: Defn.Val => v.pats.find(isProhibited)
    case v: Defn.Var => v.pats.find(isProhibited)
    case _ => None

  // A parameter of an `implicit` clause carries scalameta's `Mod.Implicit`; one of a `using` clause does not.
  private def hasNonImplicitParam(d: Defn.Def): Boolean = d.paramClauses.exists(_.values.exists(p => !p.isImplicit))

  private def checkTree(doc: SyntacticDocument): List[Diagnostic] = doc.tree.collect { case t => t }.flatMap {
    case t: Defn.Val if config.noFinalVal && t.mods.exists(_.isInstanceOf[Mod.Final]) =>
      val mod = t.mods.find(_.isInstanceOf[Mod.Final]).get
      List(Diagnostic("noFinalVal", "Final vals cause problems with incremental compilation", mod.pos))
    case t if config.noValPatterns && noValPattern(t).isDefined =>
      List(Diagnostic("noValPatterns", "Pattern matching in val assignment can result in match error, use \"_ match { ... }\" with a fallback case instead.", noValPattern(t).get.pos))
    case t: Mod.Covariant if config.noCovariantTypes =>
      List(Diagnostic("covariant", "Covariant types could lead to error-prone situations", t.pos))
    case t: Mod.Contravariant if config.noContravariantTypes =>
      List(Diagnostic("contravariant", "Contravariant types could lead to error-prone situations", t.pos))
    case t if config.noValInAbstract && abstractWithVals(t).isDefined =>
      abstractWithVals(t).get.map(v => Diagnostic("valInAbstract", "val definitions in traits/abstract classes may cause initialization bugs", v.pos))
    case t: Defn.Object if t.mods.exists(_.isInstanceOf[Mod.Implicit]) && config.noImplicitObject =>
      List(Diagnostic("implicitObject", "implicit objects may cause implicit resolution errors", t.pos))
    case t: Defn.Def if t.mods.exists(_.isInstanceOf[Mod.Implicit]) && hasNonImplicitParam(t) && config.noImplicitConversion =>
      List(Diagnostic("implicitConversion", "implicit conversions weaken type safety and always can be replaced by explicit conversions", t.pos))
    case t if config.noDefaultArgs && defaultArgs(t).isDefined =>
      defaultArgs(t).get.map(m => Diagnostic("defaultArgs", "Default args makes it hard to use methods as functions.", m.pos))
    case Term.ApplyInfix.Initial(_, t @ Term.Name("=="), _, _) if config.noUniversalEquality =>
      List(Diagnostic("==", config.noUniversalEqualityMessage, t.pos))
    case Term.Apply.Initial(Term.Select(_, t @ Term.Name("==")), _) if config.noUniversalEquality =>
      List(Diagnostic("==", config.noUniversalEqualityMessage, t.pos))
    case Term.ApplyInfix.Initial(_, t @ Term.Name("!="), _, _) if config.noUniversalEquality =>
      List(Diagnostic("!=", config.noUniversalEqualityMessage, t.pos))
    case Term.Apply.Initial(Term.Select(_, t @ Term.Name("!=")), _) if config.noUniversalEquality =>
      List(Diagnostic("!=", config.noUniversalEqualityMessage, t.pos))
    case d: Defn.Def if config.noFinalize && d.name.value == "finalize" && (d.paramss == Nil || d.paramss == List(Nil)) =>
      List(Diagnostic("noFinalize", "finalize should not be used", d.name.pos,
        "there is no guarantee that finalize will be called and\noverriding finalize incurs a performance penalty"))
    case _ => Nil
  }

  override def fix(using doc: SyntacticDocument): Patch =
    (checkTree(doc) ++ checkTokens(doc) ++ checkRegex(doc)).map(Patch.lint).asPatch
