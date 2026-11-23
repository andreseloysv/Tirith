// The model menu of a chat's box and of a bot's card. Each part lives in its own file under model-controls/; this
// entry exports what callers use and defines the <model-controls> element.
import { ModelControls } from './model-controls/element.js';

export { PROVIDERS, ownerOf, pickModel, optionsFor } from './model-controls/options.js';
export { offered, effortName, providerName, readable } from './model-controls/offer.js';
export { ModelControls };
if (!customElements.get('model-controls')) customElements.define('model-controls', ModelControls);
