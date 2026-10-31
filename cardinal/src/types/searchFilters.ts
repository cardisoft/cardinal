export type SearchFilter = Readonly<{
  id: string;
  // Label shown in the filter dropdown.
  name: string;
  // Query fragment appended to the search, e.g. `ext:mp4;mkv` or `folder:`.
  search: string;
}>;

export type FilterPreset = Readonly<{
  name: string;
  search: string;
  /** Localised label key under `filters.presets.<key>`; absent for user-created entries. */
  labelKey?: string;
}>;

const VIDEO_EXTS =
  'ext:3g2;3gp;3gp2;3gpp;amv;asf;asx;avi;bdmv;bik;d2v;divx;drc;dsa;dsm;dss;dsv;evo;f4v;flc;fli;flic;flv;hdmov;ifo;ivf;m1v;m2p;m2t;m2ts;m2v;m4v;mkv;mov;mp2v;mp4;mp4v;mpe;mpeg;mpg;mpls;mpv2;mpv4;ogm;ogv;ogx;pss;pva;qt;ram;ratdvd;rm;rmm;rmvb;roq;rpm;smil;smk;swf;tod;tp;tpr;tts;uvu;vob;vp6;webm;wm;wmp;wmv;wmx;wvx';

// Seeded on first run so the dropdown is useful before the user configures anything.
export const DEFAULT_FILTER_PRESETS: readonly FilterPreset[] = [
  // "All" needs no entry: the dropdown's empty option means "no filter".
  {
    name: 'Audio',
    search:
      'ext:aac;ac3;adt;adts;aif;aifc;aiff;amr;ape;au;cda;dts;ec3;fla;flac;lpcm;m1a;m2a;m3u;m3u8;m4a;m4b;m4p;mid;midi;mka;mp2;mp3;mpa;mpc;oga;ogg;opus;ra;rmi;snd;wav;wax;weba;wma',
    labelKey: 'audio',
  },
  {
    name: 'Archives',
    search:
      'ext:7z;ace;arj;bz2;cab;gz;gzip;jar;r00;r01;r02;r03;r04;r05;r06;r07;r08;r09;r10;r11;r12;r13;r14;r15;r16;r17;r18;r19;r20;r21;r22;r23;r24;r25;r26;r27;r28;r29;rar;tar;tgz;z;zip',
    labelKey: 'archives',
  },
  {
    name: 'Documents',
    search:
      'ext:asm;c;cc;chm;cpp;cs;css;csv;cxx;doc;docm;docx;dot;dotm;dotx;efu;epub;h;hpp;htm;html;hxx;ini;java;js;json;lua;md;mht;mhtml;mobi;odp;ods;odt;pdf;php;pl;potm;potx;ppam;pps;ppsm;ppsx;ppt;pptm;pptx;ps1xml;pssc;pub;py;rtf;sldm;sldx;sql;tsv;txt;vb;vsd;wpd;wps;wri;xlam;xls;xlsb;xlsm;xlsx;xltm;xltx;xml;xsl',
    labelKey: 'documents',
  },
  {
    name: 'Executables',
    search: 'ext:bat;cmd;exe;msi;msp;msu;ps1;scr;vbs;sh',
    labelKey: 'executables',
  },
  { name: 'Folders', search: 'folder:', labelKey: 'folders' },
  {
    name: 'Images',
    search:
      'ext:ani;apng;avif;avifs;bmp;bpg;cur;dds;gif;heic;heics;heif;heifs;hif;ico;jfi;jfif;jif;jpe;jpeg;jpg;jxl;jxr;pcx;png;psb;psd;svg;tga;tif;tiff;wdp;webp;wmf',
    labelKey: 'images',
  },
  { name: 'Videos', search: VIDEO_EXTS, labelKey: 'videos' },
];

/** The "no filtering" entry; always present and never removable. */
export const ALL_FILTER_SEARCH = '';
