// Pipelines view: the old page's .pv layout — the run list on the left (.runcard: name, project, progress bar,
// current or failing stage, stage chips, lanes, duration) and the selected run on the right as a vertical stage
// flow on the dotted canvas (.node.snode with the old stage icons, .conn, .zbar zoom and total), with the stage's
// log in the right drawer and Retry on red pipelines. Live through SSE "pipeline" and "run". Its parts are in
// web/js/views/pipelines/.
export { Pipelines } from './pipelines/view.js';
