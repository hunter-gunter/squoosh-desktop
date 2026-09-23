// Native adapters derived from Squoosh's Apache-2.0 codec wrappers.
// Ownership: every returned buffer belongs to the caller and is freed with sq_free.
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <setjmp.h>
#include <jpeglib.h>
#include <webp/encode.h>
#include <avif/avif.h>
#include <lcms2.h>
#include <sharpyuv/sharpyuv.h>
#include <sharpyuv/sharpyuv_csp.h>
#include "options.h"

void sq_free(void *p) { free(p); }
static int fail(char *error, const char *message) {
    snprintf(error, 512, "%s", message);
    return 0;
}
static int copy_result(const uint8_t *data, size_t size, uint8_t **out, size_t *len, char *error) {
    *out = malloc(size);
    if (!*out) return fail(error, "Allocation du résultat impossible");
    memcpy(*out, data, size); *len = size; return 1;
}
typedef struct { struct jpeg_error_mgr mgr; jmp_buf jump; char message[JMSG_LENGTH_MAX]; } jpeg_error;
static void jpeg_failure(j_common_ptr c) {
    jpeg_error *e = (jpeg_error *)c->err;
    c->err->format_message(c, e->message);
    longjmp(e->jump, 1);
}
int sq_jpeg(const uint8_t *pixels, uint32_t w, uint32_t h, const jpeg_options *o,
            uint8_t **out, size_t *len, char *error) {
    // Heap state remains defined after longjmp; no Rust frames are unwound by C.
    struct jpeg_compress_struct *c = calloc(1, sizeof(*c));
    jpeg_error *e = calloc(1, sizeof(*e));
    unsigned char **buffer = calloc(1, sizeof(*buffer));
    unsigned long *length = calloc(1, sizeof(*length));
    if (!c || !e || !buffer || !length) {
        free(c); free(e); free(buffer); free(length); return fail(error, "Allocation JPEG impossible");
    }
    c->err = jpeg_std_error(&e->mgr); e->mgr.error_exit = jpeg_failure;
    if (setjmp(e->jump)) {
        fail(error, e->message); jpeg_destroy_compress(c); free(*buffer);
        free(c); free(e); free(buffer); free(length); return 0;
    }
    jpeg_create_compress(c);
    jpeg_mem_dest(c, buffer, length);
    c->image_width=w; c->image_height=h; c->input_components=4; c->in_color_space=JCS_EXT_RGBA;
    jpeg_set_defaults(c);
    jpeg_set_colorspace(c, (J_COLOR_SPACE)o->color_space);
    jpeg_c_set_int_param(c, JINT_BASE_QUANT_TBL_IDX, o->quant_table);
    c->optimize_coding=o->arithmetic ? FALSE : o->optimize_coding;
    c->arith_code=o->arithmetic;
    c->smoothing_factor=o->smoothing;
    jpeg_c_set_bool_param(c, JBOOLEAN_USE_SCANS_IN_TRELLIS, o->trellis_multipass);
    jpeg_c_set_bool_param(c, JBOOLEAN_TRELLIS_EOB_OPT, o->trellis_opt_zero);
    jpeg_c_set_bool_param(c, JBOOLEAN_TRELLIS_Q_OPT, o->trellis_opt_table);
    jpeg_c_set_int_param(c, JINT_TRELLIS_NUM_LOOPS, o->trellis_loops);
    jpeg_c_set_int_param(c, JINT_DC_SCAN_OPT_MODE, 0);
    int chroma = o->separate_chroma_quality && o->color_space == JCS_YCbCr ? o->chroma_quality : o->quality;
    for (int i=0; i<NUM_QUANT_TBLS; ++i)
        c->q_scale_factor[i] = jpeg_quality_scaling(i == 0 ? o->quality : chroma);
    jpeg_default_qtables(c, o->baseline);
    if (o->color_space == JCS_YCbCr) {
        if (chroma >= 90) { c->comp_info[0].h_samp_factor=1; c->comp_info[0].v_samp_factor=1; }
        else if (chroma >= 80) { c->comp_info[0].h_samp_factor=2; c->comp_info[0].v_samp_factor=1; }
        if (!o->auto_subsample) {
            c->comp_info[0].h_samp_factor=o->chroma_subsample;
            c->comp_info[0].v_samp_factor=o->chroma_subsample;
            if (o->chroma_subsample > 2) jpeg_c_set_int_param(c, JINT_DC_SCAN_OPT_MODE, 1);
        }
    }
    if (!o->baseline && o->progressive) jpeg_simple_progression(c);
    else { c->num_scans=0; c->scan_info=NULL; }
    jpeg_start_compress(c, TRUE);
    while (c->next_scanline < h) {
        JSAMPROW row = (JSAMPROW)(pixels + (size_t)c->next_scanline*w*4);
        jpeg_write_scanlines(c, &row, 1);
    }
    jpeg_finish_compress(c);
    *out=*buffer; *len=*length;
    jpeg_destroy_compress(c); free(c); free(e); free(buffer); free(length); return 1;
}
int sq_webp(const uint8_t *pixels, uint32_t w, uint32_t h, const webp_options *o,
            uint8_t **out, size_t *len, char *error) {
    WebPConfig config;
    if (!WebPConfigInit(&config)) return fail(error, "ABI libwebp incompatible");
#define SET(name) config.name=o->name
    SET(quality); SET(target_size); SET(target_PSNR); SET(method); SET(sns_strength);
    SET(filter_strength); SET(filter_sharpness); SET(filter_type); SET(partitions);
    SET(segments); SET(pass); SET(show_compressed); SET(preprocessing); SET(autofilter);
    SET(partition_limit); SET(alpha_compression); SET(alpha_filtering); SET(alpha_quality);
    SET(lossless); SET(exact); SET(image_hint); SET(emulate_jpeg_size); SET(thread_level);
    SET(low_memory); SET(near_lossless); SET(use_delta_palette); SET(use_sharp_yuv);
#undef SET
    config.qmax=100;
    if (!WebPValidateConfig(&config)) return fail(error, "Réglages WebP invalides");
    WebPPicture pic; WebPMemoryWriter writer;
    if (!WebPPictureInit(&pic)) return fail(error, "ABI WebP incompatible");
    WebPMemoryWriterInit(&writer);
    pic.use_argb=config.lossless || config.use_sharp_yuv || config.preprocessing > 0;
    pic.width=(int)w; pic.height=(int)h; pic.writer=WebPMemoryWrite; pic.custom_ptr=&writer;
    int ok=WebPPictureImportRGBA(&pic, pixels, (int)w*4) && WebPEncode(&config, &pic);
    if (ok) ok=copy_result(writer.mem, writer.size, out, len, error);
    else snprintf(error,512,"Échec WebP (code %d)",pic.error_code);
    WebPPictureFree(&pic); WebPMemoryWriterClear(&writer); return ok;
}
// Sharp YUV through libsharpyuv directly: Debian's and vcpkg's libavif are
// built without it and would return AVIF_RESULT_NOT_IMPLEMENTED.
static avifResult sharp_yuv420(avifImage *image, const uint8_t *pixels, uint32_t w, uint32_t h) {
    avifResult status=avifImageAllocatePlanes(image,AVIF_PLANES_ALL);
    if (status!=AVIF_RESULT_OK) return status;
    const SharpYuvConversionMatrix *matrix=SharpYuvGetConversionMatrix(kSharpYuvMatrixRec601Full);
    if (!SharpYuvConvert(pixels,pixels+1,pixels+2,4,(int)w*4,8,
                         image->yuvPlanes[AVIF_CHAN_Y],(int)image->yuvRowBytes[AVIF_CHAN_Y],
                         image->yuvPlanes[AVIF_CHAN_U],(int)image->yuvRowBytes[AVIF_CHAN_U],
                         image->yuvPlanes[AVIF_CHAN_V],(int)image->yuvRowBytes[AVIF_CHAN_V],
                         8,(int)w,(int)h,matrix))
        return AVIF_RESULT_OUT_OF_MEMORY;
    for (uint32_t y=0; y<h; ++y) {
        uint8_t *row=image->alphaPlane+(size_t)y*image->alphaRowBytes;
        const uint8_t *src=pixels+(size_t)y*w*4+3;
        for (uint32_t x=0; x<w; ++x) row[x]=src[(size_t)x*4];
    }
    return AVIF_RESULT_OK;
}
int sq_avif(const uint8_t *pixels, uint32_t w, uint32_t h, const avif_options *o,
            uint8_t **out, size_t *len, char *error) {
    const avifPixelFormat formats[]={AVIF_PIXEL_FORMAT_YUV400,AVIF_PIXEL_FORMAT_YUV420,AVIF_PIXEL_FORMAT_YUV422,AVIF_PIXEL_FORMAT_YUV444};
    avifImage *image=avifImageCreate(w,h,8,formats[o->subsample]);
    avifEncoder *encoder=avifEncoderCreate(); avifRWData data=AVIF_DATA_EMPTY;
    avifResult status=AVIF_RESULT_UNKNOWN_ERROR; int ok=0;
    if (!image || !encoder) { fail(error,"Allocation AVIF impossible"); goto cleanup; }
    int lossless=o->quality==100 && (o->qualityAlpha==-1 || o->qualityAlpha==100) && o->subsample==3;
    image->matrixCoefficients=lossless ? AVIF_MATRIX_COEFFICIENTS_IDENTITY : AVIF_MATRIX_COEFFICIENTS_BT601;
    image->colorPrimaries=AVIF_COLOR_PRIMARIES_BT709;
    image->transferCharacteristics=AVIF_TRANSFER_CHARACTERISTICS_SRGB;
    avifRGBImage rgb; avifRGBImageSetDefaults(&rgb,image);
    rgb.pixels=(uint8_t *)pixels; rgb.rowBytes=w*4;
    // As in libavif, sharp YUV only applies to 4:2:0.
    status=o->enableSharpYUV && image->yuvFormat==AVIF_PIXEL_FORMAT_YUV420
        ? sharp_yuv420(image,pixels,w,h) : avifImageRGBToYUV(image,&rgb);
    if (status!=AVIF_RESULT_OK) goto codec_error;
    encoder->codecChoice=AVIF_CODEC_CHOICE_AOM;
    encoder->quality=o->quality; encoder->qualityAlpha=o->qualityAlpha<0?o->quality:o->qualityAlpha;
    encoder->maxThreads=4; encoder->tileRowsLog2=o->tileRowsLog2; encoder->tileColsLog2=o->tileColsLog2; encoder->speed=o->speed;
    if (!lossless) {
        char value[32];
#define OPTION(key,val) do { status=avifEncoderSetCodecSpecificOption(encoder,key,val); if(status!=AVIF_RESULT_OK) goto codec_error; } while(0)
        snprintf(value,sizeof(value),"%d",o->sharpness); OPTION("sharpness",value);
        OPTION("tune", o->tune==2 || (o->tune==0 && o->quality>=50) ? "ssim" : "psnr");
        if (o->chromaDeltaQ) OPTION("color:enable-chroma-deltaq","1");
        snprintf(value,sizeof(value),"%d",o->denoiseLevel); OPTION("color:denoise-noise-level",value);
#undef OPTION
    }
    status=avifEncoderWrite(encoder,image,&data);
    if(status!=AVIF_RESULT_OK) goto codec_error;
    ok=copy_result(data.data,data.size,out,len,error); goto cleanup;
codec_error:
    snprintf(error,512,"AVIF : %s (%s)",avifResultToString(status),encoder->diag.error);
cleanup:
    avifRWDataFree(&data); if(encoder) avifEncoderDestroy(encoder); if(image) avifImageDestroy(image); return ok;
}
// Apply an embedded RGB or grayscale ICC profile to RGBA8 while retaining alpha.
int sq_icc(uint8_t *pixels, size_t count, const uint8_t *icc, size_t icc_len, char *error) {
    if (icc_len > UINT32_MAX || count > UINT32_MAX) return fail(error,"Profil/image trop grand");
    cmsHPROFILE source=cmsOpenProfileFromMem(icc,(cmsUInt32Number)icc_len);
    cmsHPROFILE target=cmsCreate_sRGBProfile();
    if (!source || !target) { if(source)cmsCloseProfile(source); if(target)cmsCloseProfile(target); return fail(error,"Profil ICC invalide"); }
    cmsColorSpaceSignature space=cmsGetColorSpace(source);
    cmsHTRANSFORM transform=NULL;
    if (space==cmsSigRgbData) {
        transform=cmsCreateTransform(source,TYPE_RGBA_8,target,TYPE_RGBA_8,INTENT_PERCEPTUAL,cmsFLAGS_COPY_ALPHA);
        if(transform)cmsDoTransform(transform,pixels,pixels,(cmsUInt32Number)count);
    } else if (space==cmsSigGrayData) {
        transform=cmsCreateTransform(source,TYPE_GRAY_8,target,TYPE_RGB_8,INTENT_PERCEPTUAL,0);
        if(transform)for(size_t i=0;i<count;i++) { uint8_t gray=pixels[i*4]; cmsDoTransform(transform,&gray,pixels+i*4,1); }
    }
    if(transform)cmsDeleteTransform(transform);
    cmsCloseProfile(source); cmsCloseProfile(target);
    return transform!=NULL ? 1 : fail(error,"Profil ICC non pris en charge");
}
int sq_avif_decode(const uint8_t *data, size_t size, uint8_t **out, size_t *len,
                   uint32_t *w, uint32_t *h, int32_t *rotation, int32_t *mirror, int32_t *animated, char *error) {
    avifDecoder *decoder=avifDecoderCreate(); if(!decoder)return fail(error,"Allocation AVIF impossible");
    decoder->maxThreads=4; decoder->imageSizeLimit=40000000; decoder->imageDimensionLimit=32768;
    avifResult status=avifDecoderSetIOMemory(decoder,data,size); int ok=0;
    avifImage *view=NULL; avifRGBImage rgb; memset(&rgb,0,sizeof(rgb));
    if(status!=AVIF_RESULT_OK)goto cleanup;
    status=avifDecoderParse(decoder); if(status!=AVIF_RESULT_OK)goto cleanup;
    *animated=decoder->imageCount>1;
    status=avifDecoderNextImage(decoder); if(status!=AVIF_RESULT_OK)goto cleanup;
    avifImage *img=decoder->image;
    if(img->transferCharacteristics==AVIF_TRANSFER_CHARACTERISTICS_SMPTE2084 || img->transferCharacteristics==AVIF_TRANSFER_CHARACTERISTICS_HLG) {
        fail(error,"AVIF HDR non pris en charge par le pipeline SDR"); goto done;
    }
    *rotation=(img->transformFlags & AVIF_TRANSFORM_IROT)?img->irot.angle:0;
    *mirror=(img->transformFlags & AVIF_TRANSFORM_IMIR)?img->imir.axis:-1;
    if(img->transformFlags & AVIF_TRANSFORM_CLAP) {
        avifCropRect crop;
        if(!avifCropRectFromCleanApertureBox(&crop,&img->clap,img->width,img->height,&decoder->diag)) {
            fail(error,"Recadrage AVIF invalide"); goto done;
        }
        view=avifImageCreateEmpty();
        if(!view){fail(error,"Allocation AVIF impossible");goto done;}
        status=avifImageSetViewRect(view,img,&crop); if(status!=AVIF_RESULT_OK)goto cleanup;
        img=view;
    }
    avifRGBImageSetDefaults(&rgb,img); rgb.depth=8; rgb.format=AVIF_RGB_FORMAT_RGBA;
    status=avifRGBImageAllocatePixels(&rgb); if(status!=AVIF_RESULT_OK)goto cleanup;
    status=avifImageYUVToRGB(img,&rgb); if(status!=AVIF_RESULT_OK)goto cleanup;
    *w=rgb.width; *h=rgb.height;
    ok=copy_result(rgb.pixels,(size_t)rgb.rowBytes*rgb.height,out,len,error);
    if(ok && decoder->image->icc.size && !sq_icc(*out,(size_t)*w**h,decoder->image->icc.data,decoder->image->icc.size,error)) {free(*out);*out=NULL;ok=0;}
    goto done;
cleanup:
    fail(error,avifResultToString(status));
done:
    avifRGBImageFreePixels(&rgb); if(view)avifImageDestroy(view); avifDecoderDestroy(decoder); return ok;
}

typedef struct {
    struct jpeg_decompress_struct c;
    jpeg_error e;
    uint8_t *pixels;
    uint8_t *row;
    uint8_t *icc;
    unsigned int icc_len;
    cmsHPROFILE source;
    cmsHPROFILE target;
    cmsHTRANSFORM transform;
} jpeg_reader;
static void jpeg_reader_free(jpeg_reader *s) {
    jpeg_destroy_decompress(&s->c);free(s->row);free(s->icc);
    if(s->transform)cmsDeleteTransform(s->transform);
    if(s->source)cmsCloseProfile(s->source);
    if(s->target)cmsCloseProfile(s->target);
    free(s);
}
int sq_jpeg_decode(const uint8_t *bytes,size_t size,uint8_t **out,size_t *len,uint32_t *w,uint32_t *h,char *error) {
    jpeg_reader *s=calloc(1,sizeof(*s));if(!s)return fail(error,"Allocation JPEG impossible");
    s->c.err=jpeg_std_error(&s->e.mgr);s->e.mgr.error_exit=jpeg_failure;
    if(setjmp(s->e.jump)){fail(error,s->e.message);free(s->pixels);jpeg_reader_free(s);return 0;}
    jpeg_create_decompress(&s->c);jpeg_mem_src(&s->c,bytes,size);
    jpeg_save_markers(&s->c,JPEG_APP0+2,65535);
    jpeg_read_header(&s->c,TRUE);
    if(!s->c.image_width || !s->c.image_height || s->c.image_width>32768 || s->c.image_height>32768 || (uint64_t)s->c.image_width*s->c.image_height>40000000) {
        jpeg_reader_free(s);return fail(error,"JPEG supérieur à la limite de dimensions");
    }
    jpeg_read_icc_profile(&s->c,&s->icc,&s->icc_len);
    int cmyk=s->c.jpeg_color_space==JCS_CMYK || s->c.jpeg_color_space==JCS_YCCK;
    s->c.out_color_space=cmyk?JCS_CMYK:JCS_EXT_RGBA;
    jpeg_start_decompress(&s->c);*w=s->c.output_width;*h=s->c.output_height;*len=(size_t)*w**h*4;
    s->pixels=malloc(*len);s->row=malloc((size_t)*w*4);
    if(!s->pixels||!s->row){free(s->pixels);jpeg_reader_free(s);return fail(error,"Allocation JPEG impossible");}
    if(cmyk && s->icc_len){
        s->source=cmsOpenProfileFromMem(s->icc,s->icc_len);s->target=cmsCreate_sRGBProfile();
        if(s->source && s->target)s->transform=cmsCreateTransform(s->source,TYPE_CMYK_8,s->target,TYPE_RGBA_8,INTENT_PERCEPTUAL,0);
        if(!s->transform){free(s->pixels);jpeg_reader_free(s);return fail(error,"Profil CMJN invalide");}
    }
    while(s->c.output_scanline<s->c.output_height){
        uint8_t *dest=s->pixels+(size_t)s->c.output_scanline**w*4;
        JSAMPROW row=cmyk?s->row:dest;jpeg_read_scanlines(&s->c,&row,1);
        if(cmyk){
            if(s->c.saw_Adobe_marker)for(size_t i=0;i<(size_t)*w*4;i++)s->row[i]=255-s->row[i];
            if(s->transform)cmsDoTransform(s->transform,s->row,dest,*w);
            else for(size_t i=0;i<*w;i++)for(size_t j=0;j<3;j++)dest[i*4+j]=(uint8_t)(((255u-s->row[i*4+j])*(255u-s->row[i*4+3])+127)/255);
            for(size_t i=0;i<*w;i++)dest[i*4+3]=255;
        }
    }
    jpeg_finish_decompress(&s->c);
    if(!cmyk && s->icc_len && !sq_icc(s->pixels,(size_t)*w**h,s->icc,s->icc_len,error)) {free(s->pixels);jpeg_reader_free(s);return 0;}
    *out=s->pixels;jpeg_reader_free(s);return 1;
}
