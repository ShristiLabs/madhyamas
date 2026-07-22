import { useQuery } from '@tanstack/react-query'
import { apiGetText } from './client'

export function useCertificate() {
  return useQuery({
    queryKey: ['cert', 'ca'],
    queryFn: async () => {
      return apiGetText('/cert/ca')
    },
  })
}

export function useDownloadCertificate() {
  const { data: certificate, refetch, isFetching } = useCertificate()
  const downloadCertificate = async () => {
    if (!certificate) {
      await refetch()
      return
    }

    const blob = new Blob([certificate], { type: 'application/x-pem' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'madhyamas-ca.crt'
    a.click()
    URL.revokeObjectURL(url)
  }

  return { certificate, downloadCertificate, refetch, isFetching }
}
