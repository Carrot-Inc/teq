// Adapted from CatsImplicitsImport of scalafix-rules (https://github.com/xuwei-k/scalafix-rules, commit
// 995edb7a922724bc680158bc1cca61d568e72c88, rules/src/main/scala/fix/CatsImplicitsImport.scala).
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
// The port's local change: the setting `message` is decoded by hand where metaconfig derives it.
package teq.fix.rules

import teq.fix.*

final class CatsImplicitsImport(message: String) extends SyntacticRule("CatsImplicitsImport"):
  def this() = this(CatsImplicitsImport.defaultMessage)

  override def withConfiguration(c: Configuration): Configured[Rule] =
    c.conf.section("CatsImplicitsImport").string("message", message).map(new CatsImplicitsImport(_))

  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case t @ Import(Importer(Term.Select(Term.Name("cats"), Term.Name("implicits")), _) :: Nil) =>
        Patch.lint(Diagnostic(id = "", message = message, position = t.pos, severity = LintSeverity.Warning))
    }.asPatch

object CatsImplicitsImport:
  val defaultMessage: String = "use `cats.syntax` instead of `cats.implicits` https://github.com/typelevel/cats/issues/4138"
