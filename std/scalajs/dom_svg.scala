package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("SVGElement")
class SVGElement extends Element:
  def style: CSSStyleDeclaration = js.native
  def dataset: js.Dictionary[String] = js.native
  def ownerSVGElement: SVGSVGElement = js.native
  def viewportElement: SVGElement = js.native
  var tabIndex: Int = js.native
  def focus(options: js.Object = js.native): Unit = js.native
  def blur(): Unit = js.native
  var onclick: js.Function1[MouseEvent, Any] = js.native
  var onload: js.Function1[Event, Any] = js.native

@js.native
@JSGlobal("SVGAnimatedLength")
class SVGAnimatedLength extends js.Object:
  def baseVal: SVGLength = js.native
  def animVal: SVGLength = js.native

@js.native
@JSGlobal("SVGLength")
class SVGLength extends js.Object:
  var value: Double = js.native
  var valueAsString: String = js.native
  var valueInSpecifiedUnits: Double = js.native
  def unitType: Int = js.native

@js.native
@JSGlobal("SVGGraphicsElement")
class SVGGraphicsElement extends SVGElement:
  def getBBox(): DOMRect = js.native
  def getCTM(): js.Object = js.native
  def getScreenCTM(): js.Object = js.native

@js.native
@JSGlobal("SVGSVGElement")
class SVGSVGElement extends SVGGraphicsElement:
  def x: SVGAnimatedLength = js.native
  def y: SVGAnimatedLength = js.native
  def width: SVGAnimatedLength = js.native
  def height: SVGAnimatedLength = js.native
  def currentScale: Double = js.native
  def createSVGPoint(): js.Object = js.native
  def getElementById(elementId: String): Element = js.native

@js.native
@JSGlobal("SVGGElement")
class SVGGElement extends SVGGraphicsElement

@js.native
@JSGlobal("SVGGeometryElement")
class SVGGeometryElement extends SVGGraphicsElement:
  def getTotalLength(): Double = js.native
  def isPointInFill(point: js.Object = js.native): Boolean = js.native
  def isPointInStroke(point: js.Object = js.native): Boolean = js.native

@js.native
@JSGlobal("SVGRectElement")
class SVGRectElement extends SVGGeometryElement:
  def x: SVGAnimatedLength = js.native
  def y: SVGAnimatedLength = js.native
  def width: SVGAnimatedLength = js.native
  def height: SVGAnimatedLength = js.native
  def rx: SVGAnimatedLength = js.native
  def ry: SVGAnimatedLength = js.native

@js.native
@JSGlobal("SVGCircleElement")
class SVGCircleElement extends SVGGeometryElement:
  def cx: SVGAnimatedLength = js.native
  def cy: SVGAnimatedLength = js.native
  def r: SVGAnimatedLength = js.native

@js.native
@JSGlobal("SVGLineElement")
class SVGLineElement extends SVGGeometryElement:
  def x1: SVGAnimatedLength = js.native
  def y1: SVGAnimatedLength = js.native
  def x2: SVGAnimatedLength = js.native
  def y2: SVGAnimatedLength = js.native

@js.native
@JSGlobal("SVGPathElement")
class SVGPathElement extends SVGGeometryElement

@js.native
@JSGlobal("SVGTextContentElement")
class SVGTextContentElement extends SVGGraphicsElement:
  def getComputedTextLength(): Double = js.native
  def getNumberOfChars(): Int = js.native

@js.native
@JSGlobal("SVGTextElement")
class SVGTextElement extends SVGTextContentElement

object svg:
  type Element = SVGElement
  type SVG = SVGSVGElement
  type G = SVGGElement
  type Rect = SVGRectElement
  type Circle = SVGCircleElement
  type Line = SVGLineElement
  type Path = SVGPathElement
  type Text = SVGTextElement
  type Length = SVGLength
  type AnimatedLength = SVGAnimatedLength

object html:
  type Element = HTMLElement
  type Document = HTMLDocument
  type Anchor = HTMLAnchorElement
  type Audio = HTMLAudioElement
  type Body = HTMLBodyElement
  type BR = HTMLBRElement
  type Button = HTMLButtonElement
  type Canvas = HTMLCanvasElement
  type Collection[+E <: Element] = HTMLCollection[E]
  type Details = HTMLDetailsElement
  type Dialog = HTMLDialogElement
  type Div = HTMLDivElement
  type DList = HTMLDListElement
  type FieldSet = HTMLFieldSetElement
  type Form = HTMLFormElement
  type Head = HTMLHeadElement
  type Heading = HTMLHeadingElement
  type HR = HTMLHRElement
  type Html = HTMLHtmlElement
  type IFrame = HTMLIFrameElement
  type Image = HTMLImageElement
  type Input = HTMLInputElement
  type Label = HTMLLabelElement
  type Legend = HTMLLegendElement
  type LI = HTMLLIElement
  type Link = HTMLLinkElement
  type Media = HTMLMediaElement
  type Meta = HTMLMetaElement
  type OList = HTMLOListElement
  type OptGroup = HTMLOptGroupElement
  type Option = HTMLOptionElement
  type Paragraph = HTMLParagraphElement
  type Pre = HTMLPreElement
  type Progress = HTMLProgressElement
  type Script = HTMLScriptElement
  type Select = HTMLSelectElement
  type Span = HTMLSpanElement
  type Style = HTMLStyleElement
  type Table = HTMLTableElement
  type TableCaption = HTMLTableCaptionElement
  type TableCell = HTMLTableCellElement
  type TableRow = HTMLTableRowElement
  type TableSection = HTMLTableSectionElement
  type Template = HTMLTemplateElement
  type TextArea = HTMLTextAreaElement
  type Title = HTMLTitleElement
  type UList = HTMLUListElement
  type Unknown = HTMLUnknownElement
  type Video = HTMLVideoElement
