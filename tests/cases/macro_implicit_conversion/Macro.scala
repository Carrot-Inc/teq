// An `implicit inline def` conversion whose body is a macro, inherited by the companion of its
// result as os-lib's `PathChunk` inherits `stringPathChunkValidated`: inserted where a `String`
// stands for a `Chunk`, it expands there as any inline call does (dotty's `Inlines.needsInlining`
// over the adapted tree), so the macro runs at compile time and no method is left to call.
package mic

import scala.quoted.*
import scala.language.implicitConversions

final case class Chunk(segments: List[String])

trait ChunkMacros:
  implicit inline def validated(s: String): Chunk = ${ ChunkMacros.validate('s) }

object ChunkMacros:
  def validate(s: Expr[String])(using Quotes): Expr[Chunk] =
    s.value match
      case Some(text) if text.contains("/") => '{ Chunk(${ Expr(text.split('/').toList) }) }
      case Some(text) => '{ Chunk(List(${ Expr(text.toUpperCase) })) }
      case None => '{ Chunk(List($s)) }

object Chunk extends ChunkMacros
