// Shared bits of the page: icons, times, lane statuses and small components (ported from the old util.js).
// Each kind lives in its own file under common/; this entry exports them all, as it always did.
export { ICON, STAGE_ICON, stageIcon, Icon } from './common/icons.js';
export { ms, dur, ago, agoText, clockTime, elapsed } from './common/time.js';
export { Span, Empty } from './common/parts.js';
export { LANE, laneLabel, laneClass, ACTIVE, projectColor } from './common/lanes.js';
export { dollars, spentText } from './common/spent.js';
export { route, go } from './common/route.js';
