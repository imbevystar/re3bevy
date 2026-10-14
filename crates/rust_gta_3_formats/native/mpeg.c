#define PLM_NO_STDIO
#define PL_MPEG_IMPLEMENTATION
#include "pl_mpeg.h"

// The pinned decoder flushes its final reference picture only when the last
// coded picture was I/P. GTA III's intro ends in a B picture, so its future
// reference picture is otherwise lost. Keep this adaptation outside upstream.
plm_frame_t *rust_gta_3_mpeg_decode_video(plm_t *decoder) {
    plm_frame_t *frame = plm_decode_video(decoder);
    plm_video_t *video = decoder->video_decoder;
    if (!frame && video && plm_has_ended(decoder) &&
        video->has_reference_frame && !video->assume_no_b_frames &&
        video->picture_type == PLM_VIDEO_PICTURE_TYPE_B) {
        video->has_reference_frame = FALSE;
        frame = &video->frame_backward;
        frame->time = video->time;
        video->frames_decoded++;
        video->time = (double)video->frames_decoded / video->framerate;
    }
    return frame;
}
