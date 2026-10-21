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

// Adapted from As of typelevel-scalafix (https://github.com/typelevel/typelevel-scalafix, commit
// f5bc61f93fd290cd83f27bf277bb740f2a6a808a, modules/cats/rules/src/main/scala/org/typelevel/fix/As.scala), the
// rule `TypelevelAs`. It reads the syntax alone, as the reference does, though declared a semantic rule there.
package teq.fix.rules

import teq.fix.*

final class TypelevelAs extends SemanticRule("TypelevelAs"):
  override def fix(using doc: SemanticDocument): Patch =
    doc.tree.collect {
      // fa.map(_ => ())
      case tree @ TypelevelAs.AnonymousMap(_: Lit.Unit) => Patch.lint(TypelevelAs.diagnostic(tree, ".map(_ => ()) can be replaced by .void"))
      // fa.map(_ => 1)
      case tree @ TypelevelAs.AnonymousMap(_: Lit) => Patch.lint(TypelevelAs.diagnostic(tree, ".map(_ => f) can be replaced by .as(f)"))
      // fa.as(())
      case tree @ TypelevelAs.As(_: Lit.Unit) => Patch.lint(TypelevelAs.diagnostic(tree, ".as(()) can be replaced by .void"))
    }.asPatch

object TypelevelAs:
  object AnonymousMap:
    def unapply(term: Tree): Option[Term] = term match
      case Term.Apply.Initial(Term.Select(_, Term.Name("map")), Term.Function.Initial(List(Term.Param(_, _: Name.Placeholder, _, _)), returnType) :: Nil) =>
        Some(returnType)
      case _ => None

  object As:
    def unapply(term: Tree): Option[Term] = term match
      case Term.Apply.Initial(Term.Select(_, Term.Name("as")), List(value)) => Some(value)
      case _ => None

  def diagnostic(t: Tree, msg: String): Diagnostic = new Diagnostic:
    def message: String = msg
    def position: Position = t.pos
    override def categoryID: String = "as"
