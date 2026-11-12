/*
 * Copyright 2022 Typelevel
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// Adapted from Http4sLiteralsSyntax of typelevel-scalafix (https://github.com/typelevel/typelevel-scalafix, commit
// f5bc61f93fd290cd83f27bf277bb740f2a6a808a, modules/http4s/rules/src/main/scala/org/typelevel/fix/Http4sLiteralsSyntax.scala),
// the rule `TypelevelHttp4sLiteralsSyntax`. The port's local change: the added import is written as its text where the
// reference writes a quasiquote of it.
package teq.fix.rules

import teq.fix.*

final class TypelevelHttp4sLiteralsSyntax extends SemanticRule("TypelevelHttp4sLiteralsSyntax"):
  override def fix(using doc: SemanticDocument): Patch =
    doc.tree.collect {
      case t @ Term.Apply.Initial(TypelevelHttp4sLiteralsSyntax.Uri_unsafeFromString_M(_), List(lit @ Lit.String(_))) =>
        (Patch.replaceTree(t, "uri" + lit) + importLiteralsIfNeeded).atomic
      case t @ Term.Apply.Initial(TypelevelHttp4sLiteralsSyntax.Path_unsafeFromString_M(_), List(lit @ Lit.String(_))) =>
        (Patch.replaceTree(t, "path" + lit) + importLiteralsIfNeeded).atomic
    }.asPatch

  def importLiteralsIfNeeded(using doc: SemanticDocument): Patch =
    if !hasLiteralsImport then Patch.addGlobalImport("org.http4s.syntax.literals._") else Patch.empty

  // yuck
  def hasLiteralsImport(using doc: SemanticDocument): Boolean =
    doc.tree.collect {
      case Importer(Term.Select(Term.Select(Term.Name("org"), Term.Name("http4s")), Term.Name("implicits")), List(Importee.Wildcard())) => true
      case Importer(Term.Select(Term.Select(Term.Select(Term.Name("org"), Term.Name("http4s")), Term.Name("syntax")), Term.Name("all" | "literals")), List(Importee.Wildcard())) =>
        true
      case _ => false
    }.contains(true)

object TypelevelHttp4sLiteralsSyntax:
  val Uri_unsafeFromString_M: SymbolMatcher = SymbolMatcher.exact("org/http4s/Uri.unsafeFromString().")
  val Path_unsafeFromString_M: SymbolMatcher = SymbolMatcher.exact("org/http4s/Uri.Path.unsafeFromString().")
