// Adapted from OptionGetWarn of scalafix-rules (https://github.com/xuwei-k/scalafix-rules, commit
// 995edb7a922724bc680158bc1cca61d568e72c88, rules/src/main/scala/fix/OptionGetWarn.scala).
//
// MIT License
//
// Copyright (c) 2022 kenji yoshida
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
// The port's local changes: the setting `message` is decoded by hand where metaconfig derives it; and the rule
// reports every selection of a `get` that can throw, `scala.Option`'s own and `scala.None`'s, by the symbol of the
// `get` itself. Upstream asks the receiver's symbol for its information and reports where that names a value or a
// method of type `Option`: scalafix has an information only for a symbol its own document defines or a Java or Scala
// 2 class file declares, so it is silent on a receiver another file, a Scala 3 library or scala-library defines
// (docs/FIX.md, "The rules teq ships").
package teq.fix.rules

import teq.fix.*

final class OptionGetWarn(message: String) extends SemanticRule("OptionGetWarn"):
  def this() = this(OptionGetWarn.defaultMessage)

  override def withConfiguration(c: Configuration): Configured[Rule] =
    c.conf.section("OptionGetWarn").string("message", message).map(new OptionGetWarn(_))

  override def fix(using doc: SemanticDocument): Patch =
    doc.tree.collect {
      case Term.Select(_, get @ Term.Name("get")) if OptionGetWarn.throws(get.symbol.normalized.value) =>
        Patch.lint(Diagnostic(id = "", message = message, position = get.pos, severity = LintSeverity.Warning))
    }.asPatch

object OptionGetWarn:
  val defaultMessage: String = "Don't use Option.get"
  /** The two `get`s that can throw: `scala.Option`'s own and `scala.None`'s; `scala.Some`'s cannot. */
  def throws(normalized: String): Boolean = normalized == "scala.Option.get." || normalized == "scala.None.get."
