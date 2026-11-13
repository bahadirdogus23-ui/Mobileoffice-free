package org.mobileofficefree.app

import org.mobileofficefree.engine.SlideText

/** The smallest text box under a tap; editing a picked object cannot select another object. */
internal fun slideTextAt(shapes: List<SlideText>, x: Float, y: Float, objectId: UInt? = null): SlideText? =
    shapes.asSequence()
        .filter { objectId == null || it.id == objectId }
        .filter { x >= it.x && x <= it.x + it.width && y >= it.y && y <= it.y + it.height }
        .minByOrNull { it.width * it.height }
