// Shared with AgentsMessenger's phone chat. Keep model-controls.js, its folder model-controls/ and
// model-controls.css identical in both apps. Each part lives in its own file under model-controls/; this entry
// exports what it always did and defines the <model-controls> element.
import { ModelControls } from './model-controls/element.js';

export { PROVIDERS, ownerOf, pickModel, optionsFor } from './model-controls/options.js';
export { ModelControls };
if (!customElements.get('model-controls')) customElements.define('model-controls', ModelControls);
