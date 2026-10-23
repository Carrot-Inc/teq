package teq.fix

import scala.collection.mutable.ListBuffer

// The rule API of `teq fix` (docs/FIX.md), shaped after scalafix's `scalafix.v1` so that a scalafix rule ports with
// local changes: documents, tokens, positions, patches, diagnostics, rules and their configuration.

/** A range of a document, in UTF-16 units of its text as scalameta's; lines and columns from 0. */
final class Position(private[fix] val gen: Int, val start: Int, val end: Int):
  def startLine: Int = Native.line(gen, start)
  def startColumn: Int = Native.column(gen, start)
  def endLine: Int = Native.line(gen, end)
  def endColumn: Int = Native.column(gen, end)
  def text: String = Native.slice(gen, start, end)
  def input: Input = new Input(gen)
  override def equals(that: Any): Boolean = that match
    case p: Position => p.gen == gen && p.start == start && p.end == end
    case _ => false
  override def hashCode: Int = (gen * 31 + start) * 31 + end
  override def toString: String = Native.path(gen) + ":" + (startLine + 1) + ":" + (startColumn + 1)

object Position:
  /** The range `[start, end)` of `input`'s text. */
  def Range(input: Input, start: Int, end: Int): Position = new Position(input.gen, start, end)

/** A document's text and path. */
final class Input(private[fix] val gen: Int):
  def text: String = Native.text(gen)
  def path: String = Native.path(gen)

/** A token of the editable stream (src/fix/source.rs): every byte of the file is in one, trivia included. */
final class Token(private[fix] val gen: Int, private[fix] val idx: Int):
  /** scalameta's token class: `KwVar`, `Ident`, `Semicolon`, `Space`, `Tab`, `LF`, `Comment`, `Constant.Int`, ... */
  lazy val kind: String = Native.tokenKind(gen, idx)
  def start: Int = Native.tokenStart(gen, idx)
  def end: Int = Native.tokenEnd(gen, idx)
  def pos: Position = new Position(gen, start, end)
  def text: String = Native.slice(gen, start, end)
  def syntax: String = text
  /** A comment's text inside its delimiters, as scalameta's `Token.Comment.value`; an identifier's name, its
    * backquotes off, as `Token.Ident.value`; any other token's text. */
  def value: String =
    val t = text
    kind match
      case "Comment" => if t.startsWith("//") then t.substring(2) else t.substring(2, (t.length - 2).max(2))
      case "Ident" if t.length >= 2 && t.startsWith("`") && t.endsWith("`") => t.substring(1, t.length - 1)
      case _ => t
  def is(kind: String): Boolean = this.kind == kind
  def isKeyword: Boolean = kind.startsWith("Kw")
  def isTrivia: Boolean = kind match
    case "Space" | "Tab" | "LF" | "CRLF" | "CR" | "FF" | "Comment" | "Shebang" | "BOM" => true
    case _ => false
  override def equals(that: Any): Boolean = that match
    case t: Token => t.gen == gen && t.idx == idx
    case _ => false
  override def hashCode: Int = gen * 1000003 + idx
  override def toString: String = text

/** The kinds' codes, which the shipped rules' walks compare (`Native.tokenCodes`, `Native.tokensOf`): a code is an index of
  * the run's names (src/fix/source.rs, `TOKEN_KINDS`). */
object Token:
  private[fix] lazy val names: Array[String] = Native.tokenKindNames()
  /** A kind's code, -1 for a name no token has. */
  private[fix] def code(name: String): Int = names.indexOf(name)

/** A run of the editable stream, `[from, until)`. */
final class Tokens(private[fix] val gen: Int, val from: Int, val until: Int):
  def length: Int = until - from
  def size: Int = length
  def isEmpty: Boolean = length == 0
  def nonEmpty: Boolean = length > 0
  def apply(i: Int): Token =
    if i < 0 || i >= length then throw new IndexOutOfBoundsException(i.toString)
    new Token(gen, from + i)
  def head: Token = apply(0)
  def last: Token = apply(length - 1)
  def headOption: Option[Token] = if isEmpty then None else Some(head)
  def lastOption: Option[Token] = if isEmpty then None else Some(last)
  def toList: List[Token] =
    val out = ListBuffer.empty[Token]
    var i = from
    while i < until do
      out += new Token(gen, i)
      i += 1
    out.toList
  def iterator: Iterator[Token] = toList.iterator
  def foreach(f: Token => Unit): Unit =
    var i = from
    while i < until do
      f(new Token(gen, i))
      i += 1
  def collect[T](f: PartialFunction[Token, T]): List[T] = toList.collect(f)
  def filter(p: Token => Boolean): List[Token] = toList.filter(p)
  def find(p: Token => Boolean): Option[Token] =
    var i = from
    var found: Option[Token] = None
    while found.isEmpty && i < until do
      val t = new Token(gen, i)
      if p(t) then found = Some(t)
      i += 1
    found
  def exists(p: Token => Boolean): Boolean = find(p).isDefined
  def slice(a: Int, b: Int): Tokens = new Tokens(gen, from + a.max(0).min(length), from + b.max(a.max(0)).min(length))
  def drop(n: Int): Tokens = slice(n, length)
  def take(n: Int): Tokens = slice(0, n)
  /** The tokens after `t` in the document, as scalameta's `tokens.dropWhile(_ != t).drop(1)`. */
  def after(t: Token): Tokens = new Tokens(gen, t.idx + 1, Native.tokenCount(gen))

/** scalafix's suppression comments, as the run reads them (docs/FIX.md, "Suppressions"). */
object Suppression:
  def isAnchor(comment: String): Boolean = Native.isAnchor(comment)

/** A document as a syntactic rule reads it. */
class SyntacticDocument(private[fix] val gen: Int):
  def input: Input = new Input(gen)
  def tree: Source = Tree.of(gen, Native.root(gen)).asInstanceOf[Source]
  def tokens: Tokens = new Tokens(gen, 0, Native.tokenCount(gen))
  def path: String = Native.path(gen)

/** A document as a semantic rule reads it: the typer's answers in-process. `symbol` and `info` are the next part's
  * (docs/FIX.md); this one answers the unused imports. */
final class SemanticDocument(gen: Int) extends SyntacticDocument(gen):
  /** The importees the typer's unused-import analysis reports (`-Wunused:imports`'), or the exception that says
    * why it is unavailable: a run that cannot answer fails, never an empty answer. */
  def unusedImports: List[Importee] = Native.unusedImports(gen).toList.map(i => Tree.of(gen, i).asInstanceOf[Importee])
  /** Whether an expansion, a macro's or an inline method's, was made in the document. */
  def hasExpansions: Boolean = Native.expanded(gen)

enum LintSeverity:
  case Info, Warning, Error

/** A lint: its category (its id after the rule's name, `DisableSyntax.var`), message, position and severity. */
trait Diagnostic:
  def message: String
  def position: Position
  def severity: LintSeverity = LintSeverity.Error
  def categoryID: String = ""
  def explanation: String = ""

object Diagnostic:
  def apply(id: String, message: String, position: Position, explanation: String = "", severity: LintSeverity = LintSeverity.Error): Diagnostic =
    val (i, m, p, e, s) = (id, message, position, explanation, severity)
    new Diagnostic:
      def message: String = m
      def position: Position = p
      override def severity: LintSeverity = s
      override def categoryID: String = i
      override def explanation: String = e

/** What a rule makes of a document: edits of its tokens, of its imports, and lints, joined with `+`. */
sealed abstract class Patch:
  final def +(that: Patch): Patch = if this.isEmpty then that else if that.isEmpty then this else Patch.Concat(this, that)
  /** All of the patch or none: suppressed as one (`// scalafix:off`), refused as one. */
  final def atomic: Patch = if isEmpty then this else Patch.Atomic(this)
  def isEmpty: Boolean = false

object Patch:
  private[fix] case object Empty extends Patch:
    override def isEmpty: Boolean = true
  private[fix] final case class Concat(a: Patch, b: Patch) extends Patch
  private[fix] final case class Atomic(p: Patch) extends Patch
  private[fix] final case class Left(token: Token, text: String) extends Patch
  private[fix] final case class Right(token: Token, text: String) extends Patch
  private[fix] final case class Remove(token: Token) extends Patch
  private[fix] final case class RemoveImportee(importee: Importee) extends Patch
  private[fix] final case class AddGlobalImport(importer: String) extends Patch
  private[fix] final case class Lint(diagnostic: Diagnostic) extends Patch

  val empty: Patch = Empty
  def addLeft(token: Token, text: String): Patch = Left(token, text)
  def addRight(token: Token, text: String): Patch = Right(token, text)
  def addLeft(tree: Tree, text: String): Patch = tree.tokens.headOption.fold(empty)(t => Left(t, text))
  def addRight(tree: Tree, text: String): Patch = tree.tokens.lastOption.fold(empty)(t => Right(t, text))
  def removeToken(token: Token): Patch = Remove(token)
  def removeTokens(tokens: Tokens): Patch = removeTokens(tokens.toList)
  def removeTokens(tokens: Iterable[Token]): Patch = tokens.foldLeft(empty)((p, t) => p + Remove(t))
  def replaceToken(token: Token, text: String): Patch = Remove(token) + Right(token, text)
  /** The tree's tokens removed and `text` added right of the first, as scalafix's. */
  def replaceTree(tree: Tree, text: String): Patch =
    val ts = tree.tokens
    if ts.isEmpty then empty else removeTokens(ts) + Right(ts.head, text)
  /** The importee removed, its importer with its last importee and its import with its last importer
    * (dotty's `CheckUnused.checkImports` edits, src/typer/unused_edits.rs). */
  def removeImportee(importee: Importee): Patch = RemoveImportee(importee)
  /** `import <importer>` added after the file's last top-level import, or its package clause, unless the file has it. */
  def addGlobalImport(importer: String): Patch = AddGlobalImport(importer)
  def lint(diagnostic: Diagnostic): Patch = Lint(diagnostic)
  def fromIterable(ps: Iterable[Patch]): Patch = ps.foldLeft(empty)(_ + _)

  /** Hands the patch to `teq fix`, rule `rule` of the run. A token, an importee and a position go with the
    * generation of the document they were made from, which the run checks as an accessor checks it: a patch made
    * from another document fails the rule. */
  private[fix] def emit(p: Patch, gen: Int, rule: Int, group: Int): Unit = p match
    case Empty => ()
    case Concat(a, b) =>
      emit(a, gen, rule, group)
      emit(b, gen, rule, group)
    case Atomic(inner) => emit(inner, gen, rule, if group != 0 then group else Native.newGroup())
    case Left(t, s) => Native.addLeft(t.gen, rule, group, t.idx, s)
    case Right(t, s) => Native.addRight(t.gen, rule, group, t.idx, s)
    case Remove(t) => Native.removeToken(t.gen, rule, group, t.idx, "")
    case RemoveImportee(i) => Native.removeImportee(i.gen, rule, group, i.idx)
    case AddGlobalImport(i) => Native.addGlobalImport(gen, rule, group, i)
    case Lint(d) =>
      val severity = d.severity match
        case LintSeverity.Info => 0
        case LintSeverity.Warning => 1
        case LintSeverity.Error => 2
      val at = d.position
      Native.lint(at.gen, rule, group, d.categoryID, d.message, d.explanation, severity, at.start, at.end)

extension (patches: Iterable[Patch])
  def asPatch: Patch = Patch.fromIterable(patches)

extension (patch: Option[Patch])
  def asPatch: Patch = patch.getOrElse(Patch.empty)

/** What a rule's settings decode to, or why they do not. */
enum Configured[+T]:
  case Ok(value: T)
  case NotOk(message: String)

  def map[U](f: T => U): Configured[U] = this match
    case Ok(v) => Ok(f(v))
    case NotOk(m) => NotOk(m)
  def flatMap[U](f: T => Configured[U]): Configured[U] = this match
    case Ok(v) => f(v)
    case NotOk(m) => NotOk(m)
  def get: T = this match
    case Ok(v) => v
    case NotOk(m) => throw new IllegalArgumentException(m)

object Configured:
  /** Every value, or the messages of those that are not. */
  def all[T](cs: List[Configured[T]]): Configured[List[T]] =
    val errors = cs.collect { case NotOk(m) => m }
    if errors.nonEmpty then NotOk(errors.mkString("\n")) else Ok(cs.collect { case Ok(v) => v })

/** A setting of the configuration, a node of what `teq fix` read (src/fix/config.rs). Reading a key marks it
  * read; a key under a configured rule's section that no decoding read is an unknown setting. */
final class Conf(private[fix] val id: Int):
  /** `string`, `number`, `boolean`, `null`, `list` or `object`. */
  def kind: String = if id < 0 then "object" else Native.confKind(id) match
    case 0 => "string"
    case 1 => "number"
    case 2 => "boolean"
    case 3 => "null"
    case 4 => "list"
    case _ => "object"
  def place: String = if id < 0 then "<none>" else Native.confPlace(id)
  def get(key: String): Option[Conf] =
    val f = if id < 0 then -1 else Native.confField(id, key)
    if f < 0 then None else Some(new Conf(f))
  def keys: List[String] = if id < 0 then Nil else Native.confKeys(id).toList
  /** A list's items; a setting that is no list throws (`asList` says so as a configuration error). */
  def items: List[Conf] =
    if id < 0 then Nil
    else if kind != "list" then throw new IllegalArgumentException(place + ": expected a list, found a " + kind)
    else Native.confItems(id).toList.map(new Conf(_))
  /** A list's items, or why the setting is none: metaconfig's collection decoders read a list alone. */
  def asList: Configured[List[Conf]] = kind match
    case "list" => Configured.Ok(items)
    case k => Configured.NotOk(place + ": expected a list, found a " + k)
  def asString: Configured[String] = kind match
    case "string" | "number" | "boolean" => Configured.Ok(Native.confString(id))
    case k => Configured.NotOk(place + ": expected a string, found a " + k)
  def asBoolean: Configured[Boolean] = kind match
    case "boolean" => Configured.Ok(Native.confString(id) == "true")
    case k => Configured.NotOk(place + ": expected a boolean, found a " + k)
  def asInt: Configured[Int] = kind match
    case "number" if Native.confString(id).forall(c => c.isDigit || c == '-') => Configured.Ok(Native.confString(id).toInt)
    case k => Configured.NotOk(place + ": expected an integer, found a " + k)
  def asStrings: Configured[List[String]] = asList.flatMap(l => Configured.all(l.map(_.asString)))
  def asRegex: Configured[scala.util.matching.Regex] = asString.flatMap { s =>
    try
      java.util.regex.Pattern.compile(s)
      Configured.Ok(new scala.util.matching.Regex(s))
    catch case e: Exception => Configured.NotOk(place + ": " + e.getMessage)
  }
  def string(key: String, default: String): Configured[String] = get(key).fold(Configured.Ok(default))(_.asString)
  def boolean(key: String, default: Boolean): Configured[Boolean] = get(key).fold(Configured.Ok(default))(_.asBoolean)
  def int(key: String, default: Int): Configured[Int] = get(key).fold(Configured.Ok(default))(_.asInt)
  def strings(key: String, default: List[String]): Configured[List[String]] = get(key).fold(Configured.Ok(default))(_.asStrings)
  /** The section `key`, an empty one where the configuration has none. */
  def section(key: String): Conf = get(key).getOrElse(Conf.empty)

object Conf:
  /** A section with no keys. */
  val empty: Conf = new Conf(-1)

/** What a rule's `withConfiguration` reads: the whole configuration, its rule's section by its name. */
final class Configuration(val conf: Conf)

/** A rule: its name, its configuration, and what it makes of a document. */
abstract class Rule(val name: String):
  def description: String = ""
  /** scalafix's metadata for its documentation: a linter, a rewrite, a rule subject to breaking changes. */
  def isLinter: Boolean = false
  def isRewrite: Boolean = false
  def isExperimental: Boolean = false
  def withConfiguration(config: Configuration): Configured[Rule] = Configured.Ok(this)
  private[fix] def run(doc: SemanticDocument): Patch

abstract class SyntacticRule(name: String) extends Rule(name):
  def fix(using doc: SyntacticDocument): Patch
  private[fix] final def run(doc: SemanticDocument): Patch = fix(using doc)

abstract class SemanticRule(name: String) extends Rule(name):
  def fix(using doc: SemanticDocument): Patch
  private[fix] final def run(doc: SemanticDocument): Patch = fix(using doc)

/** What `teq fix` calls (src/fix/engine.rs). */
object Driver:
  /** The rules teq ships. */
  def shipped(): Array[Rule] =
    Array(new rules.DisableSyntax(), new rules.CatsImplicitsImport(), new rules.TypelevelAs(), new rules.EmptyCollectionsUnified(), new rules.NoUnusedImports())
  def name(rule: Rule): String = rule.name
  def isSemantic(rule: Rule): Boolean = rule.isInstanceOf[SemanticRule]
  /** The rule configured, or the exception that says why it cannot be. */
  def configure(rule: Rule): Rule = rule.withConfiguration(new Configuration(new Conf(0))) match
    case Configured.Ok(r) => r
    case Configured.NotOk(m) => throw new IllegalArgumentException(m)
  /** Every rule over the document `gen`, or its semantic ones alone: each one's patch handed to the run, each one's
    * failure told. */
  def run(rules: Array[Rule], gen: Int, semanticOnly: Boolean): Unit =
    val doc = new SemanticDocument(gen)
    var i = 0
    while i < rules.length do
      if !semanticOnly || isSemantic(rules(i)) then
        try Patch.emit(rules(i).run(doc), gen, i, 0)
        catch case e: Throwable => Native.failed(gen, i, e.toString)
      i += 1
