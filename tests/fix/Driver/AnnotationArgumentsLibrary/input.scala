/*
rules = ["file:../../support/AnnotationNames.scala"]
*/
// An annotation's argument is an occurrence whether or not anything reads the annotation: the codec of `Parcel` is
// derived by a macro that reads `jsonMemberNames`, `Crate` derives nothing, and scalac's SemanticDB has the argument's
// name at both.
package annotated

import zio.json.*

@jsonMemberNames(PascalCase) final case class Parcel(firstPart: Int, secondPart: String) derives JsonCodec // assert: AnnotationNames.at

@jsonMemberNames(SnakeCase) final case class Crate(firstPart: Int) // assert: AnnotationNames.at

object Members:
  def shown: String = Parcel(1, "x").toJson
