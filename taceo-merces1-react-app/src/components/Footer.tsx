export default function Footer() {
  return (
    <footer className="w-full border-t border-zinc-200 mt-5">
      <div className="grid grid-cols-1 gap-x-4 gap-y-5 px-4 pt-5 pb-5 md:grid-cols-3 md:px-8">
        <div className="w-full text-center text-xs font-medium text-zinc-500 md:text-left">
          [Don&apos;t] share your data.
        </div>
        <p className="w-full text-center text-xs font-medium text-zinc-500">
          © {new Date().getFullYear()}{" "}
          <a href="https://taceo.io/" className="hover:underline">
            TACEO
          </a>{" "}
          |{" "}
          <a
            href="https://taceo.io/disclaimer/"
            target="_blank"
            rel="noopener noreferrer"
            className="underline"
          >
            Disclaimer
          </a>
        </p>
        <div className="flex items-center justify-center gap-4 md:justify-end">
          <a target="_blank" rel="noopener noreferrer" title="Taceo on GitHub" href="https://github.com/TaceoLabs">
            <svg width="14" height="13" viewBox="0 0 14 13" fill="none" xmlns="http://www.w3.org/2000/svg">
              <g clipPath="url(#gh)">
                <path fillRule="evenodd" clipRule="evenodd" d="M7.006.078C3.132.078 0 2.946 0 6.495c0 2.837 2.007 5.238 4.79 6.088.348.064.476-.138.476-.308 0-.149-.012-.659-.012-1.19C3.306 11.468 2.9 10.32 2.9 10.32c-.313-.744-.777-.935-.777-.935-.638-.393.046-.393.046-.393.708.042 1.079.651 1.079.651.626.977 1.635.7 2.041.53.058-.413.244-.7.44-.86-1.553-.148-3.189-.7-3.189-3.166 0-.701.278-1.275.72-1.721-.07-.16-.314-.818.303-1.7 0 0 .591-.17 1.925.659A6.7 6.7 0 0 1 7.006 3.18c.59.001 1.178.072 1.751.213 1.334-.829 1.926-.659 1.926-.659.617.882.373 1.54.303 1.7.452.446.718 1.02.718 1.721 0 2.465-1.635 3.007-3.201 3.166.255.202.476.585.476 1.19 0 .86-.012 1.552-.012 1.764 0 .17.128.372.476.308C11.993 11.733 14 9.332 14 6.495 14.011 2.946 10.868.078 7.006.078Z" fill="#737373" />
              </g>
              <defs><clipPath id="gh"><rect width="14" height="13" fill="white" /></clipPath></defs>
            </svg>
          </a>
          <a target="_blank" rel="noopener noreferrer" title="Taceo on Discord" href="https://discord.com/invite/sgasCRPRUd">
            <svg xmlns="http://www.w3.org/2000/svg" width="15" height="11" fill="none">
              <path fill="#737373" d="M12.447 1.37A12.6 12.6 0 0 0 9.58.577a.05.05 0 0 0-.047.02 6 6 0 0 0-.357.655 12 12 0 0 0-3.22 0 6 6 0 0 0-.363-.656.05.05 0 0 0-.046-.02A12.6 12.6 0 0 0 2.68 1.37a.04.04 0 0 0-.02.015C.836 3.825.336 6.205.58 8.555a.04.04 0 0 0 .018.03 12.3 12.3 0 0 0 3.518 1.589.05.05 0 0 0 .05-.015c.27-.331.51-.68.719-1.046.012-.022 0-.047-.025-.056a8 8 0 0 1-1.098-.468.038.038 0 0 1-.005-.067c.073-.05.146-.1.218-.153a.05.05 0 0 1 .046-.005c2.305.94 4.8.94 7.08 0a.05.05 0 0 1 .044.005l.22.153a.038.038 0 0 1-.018.052 7.3 7.3 0 0 1-1.1.468c-.024.008-.035.034-.023.056.212.366.452.715.72 1.046a.05.05 0 0 0 .049.015 12.2 12.2 0 0 0 3.523-1.59.04.04 0 0 0 .018-.03c.293-2.716-.492-5.076-2.083-7.168a.03.03 0 0 0-.018-.015ZM5.229 7.125c-.694 0-1.266-.57-1.266-1.27 0-.7.56-1.27 1.266-1.27.71 0 1.277.575 1.266 1.27 0 .7-.56 1.27-1.266 1.27Zm4.68 0c-.693 0-1.265-.57-1.265-1.27 0-.7.56-1.27 1.266-1.27.71 0 1.277.575 1.265 1.27 0 .7-.554 1.27-1.265 1.27Z" />
            </svg>
          </a>
          <a target="_blank" rel="noopener noreferrer" title="Taceo on LinkedIn" href="https://www.linkedin.com/company/taceoio/">
            <svg xmlns="http://www.w3.org/2000/svg" width="15" height="13" fill="none">
              <g clipPath="url(#li)">
                <path fill="#737373" d="M13.088 0H1.083C.508 0 .043.406.043.908v10.777c0 .502.465.91 1.04.91h12.005c.575 0 1.042-.408 1.042-.907V.908C14.13.406 13.663 0 13.088 0ZM4.223 10.733H2.132V4.721h2.09l.001 6.012ZM3.177 3.902c-.671 0-1.213-.485-1.213-1.083s.542-1.082 1.213-1.082c.669 0 1.21.484 1.21 1.082 0 .596-.541 1.083-1.21 1.083Zm8.87 6.831H9.96V7.811c0-.697-.013-1.595-1.086-1.595-1.087 0-1.252.76-1.252 1.545v2.972H5.535V4.721h2.003v.821h.028c.278-.472.96-.971 1.975-.971 2.116 0 2.507 1.245 2.507 2.863l-.001 3.299Z" />
              </g>
              <defs><clipPath id="li"><path fill="#fff" d="M0 0h15v13H0z" /></clipPath></defs>
            </svg>
          </a>
          <a target="_blank" rel="noopener noreferrer" title="Taceo on X" href="https://x.com/TACEO_IO">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="11" fill="none">
              <g clipPath="url(#x)">
                <path fill="#737373" d="M10.735.05h1.98L8.387 4.47l5.09 6.017H9.492l-3.12-3.648L2.8 10.487H.82l4.626-4.728L.564.05H4.65l2.821 3.335L10.735.05Zm-.695 9.377h1.097L4.053 1.055H2.876l7.164 8.372Z" />
              </g>
              <defs><clipPath id="x"><path fill="#fff" d="M0 0h14v11H0z" /></clipPath></defs>
            </svg>
          </a>
        </div>
      </div>
    </footer>
  );
}
